//!

use super::{branch::branch_set_upstream, CommitId};
use crate::{
    error::{Error, Result},
    sync::cred::BasicAuthCredential,
    sync::utils,
};
use crossbeam_channel::Sender;
use git2::{
    Cred, Error as GitError, FetchOptions, PackBuilderStage,
    PushOptions, RemoteCallbacks, Repository,
};
use scopetime::scope_time;

///
#[derive(Debug, Clone)]
pub enum ProgressNotification {
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
pub fn get_remotes(repo_path: &str) -> Result<Vec<String>> {
    scope_time!("get_remotes");

    let repo = utils::repo(repo_path)?;
    let remotes = repo.remotes()?;
    let remotes: Vec<String> =
        remotes.iter().flatten().map(String::from).collect();

    Ok(remotes)
}

///
pub fn get_first_remote(repo_path: &str) -> Result<String> {
    let repo = utils::repo(repo_path)?;
    get_first_remote_in_repo(&repo)
}

///
pub(crate) fn get_first_remote_in_repo(
    repo: &Repository,
) -> Result<String> {
    scope_time!("get_remotes");

    let remotes = repo.remotes()?;

    let first_remote = remotes
        .iter()
        .next()
        .flatten()
        .map(String::from)
        .ok_or_else(|| Error::Generic("no remote found".into()))?;

    Ok(first_remote)
}

///
pub fn fetch_origin(repo_path: &str, branch: &str) -> Result<usize> {
    scope_time!("fetch_origin");

    let repo = utils::repo(repo_path)?;
    let mut remote =
        repo.find_remote(&get_first_remote_in_repo(&repo)?)?;

    let mut options = FetchOptions::new();
    options.remote_callbacks(remote_callbacks(None, None));

    remote.fetch(&[branch], Some(&mut options), None)?;

    Ok(remote.stats().received_bytes())
}

///
pub fn push(
    repo_path: &str,
    remote: &str,
    branch: &str,
    basic_credential: Option<BasicAuthCredential>,
    progress_sender: Sender<ProgressNotification>,
) -> Result<()> {
    scope_time!("push");

    let repo = utils::repo(repo_path)?;
    let mut remote = repo.find_remote(remote)?;

    let mut options = PushOptions::new();

    options.remote_callbacks(remote_callbacks(
        Some(progress_sender),
        basic_credential,
    ));
    options.packbuilder_parallelism(0);

    let branch_name = format!("refs/heads/{}", branch);

    remote.push(&[branch_name.as_str()], Some(&mut options))?;

    branch_set_upstream(&repo, branch)?;

    Ok(())
}

fn remote_callbacks<'a>(
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
    use super::*;
    use crate::sync::tests::debug_cmd_print;
    use tempfile::TempDir;

    #[test]
    fn test_smoke() {
        let td = TempDir::new().unwrap();

        debug_cmd_print(
            td.path().as_os_str().to_str().unwrap(),
            "git clone https://github.com/extrawurst/brewdump.git",
        );

        let repo_path = td.path().join("brewdump");
        let repo_path = repo_path.as_os_str().to_str().unwrap();

        let remotes = get_remotes(repo_path).unwrap();

        assert_eq!(remotes, vec![String::from("origin")]);

        fetch_origin(repo_path, "master").unwrap();
    }

    #[test]
    fn test_first_remote() {
        let td = TempDir::new().unwrap();

        debug_cmd_print(
            td.path().as_os_str().to_str().unwrap(),
            "git clone https://github.com/extrawurst/brewdump.git",
        );

        debug_cmd_print(
            td.path().as_os_str().to_str().unwrap(),
            "cd brewdump && git remote add second https://github.com/extrawurst/brewdump.git",
        );

        let repo_path = td.path().join("brewdump");
        let repo_path = repo_path.as_os_str().to_str().unwrap();

        let remotes = get_remotes(repo_path).unwrap();

        assert_eq!(
            remotes,
            vec![String::from("origin"), String::from("second")]
        );

        let first = get_first_remote_in_repo(
            &utils::repo(repo_path).unwrap(),
        )
        .unwrap();
        assert_eq!(first, String::from("origin"));
    }
}
