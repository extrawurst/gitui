use super::utils;
use crate::{
    error::Result,
    sync::{
        branch::branch_set_upstream, cred::BasicAuthCredential,
        CommitId,
    },
};
use crossbeam_channel::Sender;
use git2::{
    Cred, Error as GitError, PackBuilderStage, PushOptions,
    RemoteCallbacks,
};
use scopetime::scope_time;

///
#[derive(Debug, Clone)]
pub(crate) enum ProgressNotification {
    ///
    UpdateTips {
        ///
        name: String,
        ///
        a: CommitId,
        ///
        b: CommitId,
    },
    ///
    Transfer {
        ///
        objects: usize,
        ///
        total_objects: usize,
    },
    ///
    PushTransfer {
        ///
        current: usize,
        ///
        total: usize,
        ///
        bytes: usize,
    },
    ///
    Packing {
        ///
        stage: PackBuilderStage,
        ///
        total: usize,
        ///
        current: usize,
    },
    ///
    Done,
}

///
pub(crate) fn push(
    repo_path: &str,
    remote: &str,
    branch: &str,
    force: bool,
    basic_credential: Option<BasicAuthCredential>,
    progress_sender: Option<Sender<ProgressNotification>>,
) -> Result<()> {
    scope_time!("push");

    let repo = utils::repo(repo_path)?;
    let mut remote = repo.find_remote(remote)?;

    let mut options = PushOptions::new();

    options.remote_callbacks(remote_callbacks(
        progress_sender,
        basic_credential,
    ));
    options.packbuilder_parallelism(0);

    let branch_name = format!("refs/heads/{}", branch);
    if force {
        remote.push(
            &[String::from("+") + &branch_name],
            Some(&mut options),
        )?;
    } else {
        remote.push(&[branch_name.as_str()], Some(&mut options))?;
    }
    branch_set_upstream(&repo, branch)?;

    Ok(())
}

pub(crate) fn remote_callbacks<'a>(
    sender: Option<Sender<ProgressNotification>>,
    basic_credential: Option<BasicAuthCredential>,
) -> RemoteCallbacks<'a> {
    let mut callbacks = RemoteCallbacks::new();
    let sender_clone = sender.clone();
    callbacks.push_transfer_progress(move |current, total, bytes| {
        log::debug!("progress: {}/{} ({} B)", current, total, bytes,);

        sender_clone.clone().map(|sender| {
            sender.send(ProgressNotification::PushTransfer {
                current,
                total,
                bytes,
            })
        });
    });

    let sender_clone = sender.clone();
    callbacks.update_tips(move |name, a, b| {
        log::debug!("update tips: '{}' [{}] [{}]", name, a, b);

        sender_clone.clone().map(|sender| {
            sender.send(ProgressNotification::UpdateTips {
                name: name.to_string(),
                a: a.into(),
                b: b.into(),
            })
        });
        true
    });

    let sender_clone = sender.clone();
    callbacks.transfer_progress(move |p| {
        log::debug!(
            "transfer: {}/{}",
            p.received_objects(),
            p.total_objects()
        );

        sender_clone.clone().map(|sender| {
            sender.send(ProgressNotification::Transfer {
                objects: p.received_objects(),
                total_objects: p.total_objects(),
            })
        });
        true
    });

    callbacks.pack_progress(move |stage, current, total| {
        log::debug!("packing: {:?} - {}/{}", stage, current, total);

        sender.clone().map(|sender| {
            sender.send(ProgressNotification::Packing {
                stage,
                total,
                current,
            })
        });
    });

    let mut first_call_to_credentials = true;
    // This boolean is used to avoid multiple calls to credentials callback.
    // If credentials are bad, we don't ask the user to re-fill their creds. We push an error and they will be able to restart their action (for example a push) and retype their creds.
    // This behavior is explained in a issue on git2-rs project : https://github.com/rust-lang/git2-rs/issues/347
    // An implementation reference is done in cargo : https://github.com/rust-lang/cargo/blob/9fb208dddb12a3081230a5fd8f470e01df8faa25/src/cargo/sources/git/utils.rs#L588
    // There is also a guide about libgit2 authentication : https://libgit2.org/docs/guides/authentication/
    callbacks.credentials(
        move |url, username_from_url, allowed_types| {
            log::debug!(
                "creds: '{}' {:?} ({:?})",
                url,
                username_from_url,
                allowed_types
            );
            if first_call_to_credentials {
                first_call_to_credentials = false;
            } else {
                return Err(GitError::from_str("Bad credentials."));
            }

            match &basic_credential {
                _ if allowed_types.is_ssh_key() => {
                    match username_from_url {
                        Some(username) => {
                            Cred::ssh_key_from_agent(username)
                        }
                        None => Err(GitError::from_str(
                            " Couldn't extract username from url.",
                        )),
                    }
                }
                Some(BasicAuthCredential {
                    username: Some(user),
                    password: Some(pwd),
                }) if allowed_types.is_user_pass_plaintext() => {
                    Cred::userpass_plaintext(&user, &pwd)
                }
                Some(BasicAuthCredential {
                    username: Some(user),
                    password: _,
                }) if allowed_types.is_username() => {
                    Cred::username(user)
                }
                _ if allowed_types.is_default() => Cred::default(),
                _ => Err(GitError::from_str(
                    "Couldn't find credentials",
                )),
            }
        },
    );

    callbacks
}

#[cfg(test)]
mod tests {
    use git2::Repository;

    use super::*;
    use crate::sync::{
        self,
        tests::{get_commit_ids, repo_init, repo_init_bare},
    };
    use std::{fs::File, io::Write, path::Path};

    #[test]
    fn test_force_push() {
        // This test mimics the scenario of 2 people having 2
        // local branches and both modifying the same file then
        // both pushing, sequentially

        let (tmp_repo_dir, repo) = repo_init().unwrap();
        let (tmp_other_repo_dir, other_repo) = repo_init().unwrap();
        let (tmp_upstream_dir, _) = repo_init_bare().unwrap();

        repo.remote(
            "origin",
            tmp_upstream_dir.path().to_str().unwrap(),
        )
        .unwrap();

        other_repo
            .remote(
                "origin",
                tmp_upstream_dir.path().to_str().unwrap(),
            )
            .unwrap();

        let tmp_repo_file_path =
            tmp_repo_dir.path().join("temp_file.txt");
        let mut tmp_repo_file =
            File::create(tmp_repo_file_path).unwrap();
        writeln!(tmp_repo_file, "TempSomething").unwrap();

        sync::commit(
            tmp_repo_dir.path().to_str().unwrap(),
            "repo_1_commit",
        )
        .unwrap();

        push(
            tmp_repo_dir.path().to_str().unwrap(),
            "origin",
            "master",
            false,
            None,
            None,
        )
        .unwrap();

        let tmp_other_repo_file_path =
            tmp_other_repo_dir.path().join("temp_file.txt");
        let mut tmp_other_repo_file =
            File::create(tmp_other_repo_file_path).unwrap();
        writeln!(tmp_other_repo_file, "TempElse").unwrap();

        sync::commit(
            tmp_other_repo_dir.path().to_str().unwrap(),
            "repo_2_commit",
        )
        .unwrap();

        // Attempt a normal push,
        // should fail as branches diverged
        assert_eq!(
            push(
                tmp_other_repo_dir.path().to_str().unwrap(),
                "origin",
                "master",
                false,
                None,
                None,
            )
            .is_err(),
            true
        );

        // Attempt force push,
        // should work as it forces the push through
        assert_eq!(
            push(
                tmp_other_repo_dir.path().to_str().unwrap(),
                "origin",
                "master",
                true,
                None,
                None,
            )
            .is_err(),
            false
        );
    }

    #[test]
    fn test_force_push_rewrites_history() {
        // This test mimics the scenario of 2 people having 2
        // local branches and both modifying the same file then
        // both pushing, sequentially

        let (tmp_repo_dir, repo) = repo_init().unwrap();
        let (tmp_other_repo_dir, other_repo) = repo_init().unwrap();
        let (tmp_upstream_dir, upstream) = repo_init_bare().unwrap();

        repo.remote(
            "origin",
            tmp_upstream_dir.path().to_str().unwrap(),
        )
        .unwrap();

        other_repo
            .remote(
                "origin",
                tmp_upstream_dir.path().to_str().unwrap(),
            )
            .unwrap();

        let tmp_repo_file_path =
            tmp_repo_dir.path().join("temp_file.txt");
        let mut tmp_repo_file =
            File::create(tmp_repo_file_path).unwrap();
        writeln!(tmp_repo_file, "TempSomething").unwrap();

        sync::stage_add_file(
            tmp_repo_dir.path().to_str().unwrap(),
            Path::new("temp_file.txt"),
        )
        .unwrap();

        let repo_1_commit = sync::commit(
            tmp_repo_dir.path().to_str().unwrap(),
            "repo_1_commit",
        )
        .unwrap();

        //NOTE: make sure the commit actually contains that file
        assert_eq!(
            sync::get_commit_files(
                tmp_repo_dir.path().to_str().unwrap(),
                repo_1_commit
            )
            .unwrap()[0]
                .path,
            String::from("temp_file.txt")
        );

        let commits = get_commit_ids(&repo, 1);
        assert!(commits.contains(&repo_1_commit));

        push(
            tmp_repo_dir.path().to_str().unwrap(),
            "origin",
            "master",
            false,
            None,
            None,
        )
        .unwrap();

        let tmp_other_repo_file_path =
            tmp_other_repo_dir.path().join("temp_file.txt");
        let mut tmp_other_repo_file =
            File::create(tmp_other_repo_file_path).unwrap();
        writeln!(tmp_other_repo_file, "TempElse").unwrap();

        sync::stage_add_file(
            tmp_other_repo_dir.path().to_str().unwrap(),
            Path::new("temp_file.txt"),
        )
        .unwrap();

        let repo_2_commit = sync::commit(
            tmp_other_repo_dir.path().to_str().unwrap(),
            "repo_2_commit",
        )
        .unwrap();

        let repo_2_parent = other_repo
            .find_commit(repo_2_commit.into())
            .unwrap()
            .parents()
            .next()
            .unwrap()
            .id();

        let commits = get_commit_ids(&other_repo, 1);
        assert!(commits.contains(&repo_2_commit));

        // Attempt a normal push,
        // should fail as branches diverged
        assert_eq!(
            push(
                tmp_other_repo_dir.path().to_str().unwrap(),
                "origin",
                "master",
                false,
                None,
                None,
            )
            .is_err(),
            true
        );

        // Check that the other commit is not in upstream,
        // a normal push would not rewrite history
        let commits = get_commit_ids(&upstream, 1);
        assert!(!commits.contains(&repo_2_commit));

        // Attempt force push,
        // should work as it forces the push through

        push(
            tmp_other_repo_dir.path().to_str().unwrap(),
            "origin",
            "master",
            true,
            None,
            None,
        )
        .unwrap();

        let commits = get_commit_ids(&upstream, 1);
        assert!(commits.contains(&repo_2_commit));

        let new_upstream_parent =
            Repository::init_bare(tmp_upstream_dir.path())
                .unwrap()
                .find_commit(repo_2_commit.into())
                .unwrap()
                .parents()
                .next()
                .unwrap()
                .id();
        assert_eq!(new_upstream_parent, repo_2_parent,);
    }
}
