//!

use super::{push::remote_callbacks, utils};
use crate::{error::Result, sync::cred::BasicAuthCredential};
use crossbeam_channel::Sender;
use git2::Direction;
use scopetime::scope_time;

pub(crate) struct PushTagsProgress {
    pub pushed: usize,
    pub total: usize,
}

///
pub(crate) fn push_tags(
    repo_path: &str,
    remote: &str,
    basic_credential: Option<BasicAuthCredential>,
    progress_sender: Option<Sender<PushTagsProgress>>,
) -> Result<()> {
    scope_time!("push_tags");

    let repo = utils::repo(repo_path)?;
    let mut remote = repo.find_remote(remote)?;
    log::debug!("push_tags: {}", remote.connected());
    remote.connect_auth(
        Direction::Push,
        Some(remote_callbacks(None, basic_credential.clone())),
        None,
    )?;
    log::debug!("push_tags connected: {}", remote.connected());

    let tags = repo.tag_names(None)?;
    let total = tags.len();
    log::debug!("start push tags: {}", total);
    for (idx, e) in tags.into_iter().enumerate() {
        if let Some(name) = e {
            log::debug!("next tag: [{}]{}", idx, name);
            let refspec = format!("refs/tags/{}", name);

            log::debug!(
                "push tag: {}/{} ({})",
                idx,
                total,
                remote.connected()
            );
            remote.push(&[refspec.as_str()], None)?;
        }

        log::debug!("send progress: {}/{}", idx, total);
        progress_sender.as_ref().map(|sender| {
            sender.send(PushTagsProgress { pushed: idx, total })
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::{
        self,
        remotes::{fetch_origin, push::push},
        tests::{repo_clone, repo_init_bare},
        CommitId,
    };
    use git2::Repository;
    use std::{fs::File, io::Write, path::Path};

    // write, stage and commit a file
    fn write_commit_file(
        repo: &Repository,
        file: &str,
        content: &str,
        commit_name: &str,
    ) -> CommitId {
        File::create(
            repo.workdir().unwrap().join(file).to_str().unwrap(),
        )
        .unwrap()
        .write_all(content.as_bytes())
        .unwrap();

        sync::stage_add_file(
            repo.workdir().unwrap().to_str().unwrap(),
            Path::new(file),
        )
        .unwrap();

        sync::commit(
            repo.workdir().unwrap().to_str().unwrap(),
            commit_name,
        )
        .unwrap()
    }

    #[test]
    fn test_push_pull_tags() {
        let (r1_dir, _repo) = repo_init_bare().unwrap();
        let r1_dir = r1_dir.path().to_str().unwrap();

        let (clone1_dir, clone1) = repo_clone(r1_dir).unwrap();

        let clone1_dir = clone1_dir.path().to_str().unwrap();

        let (clone2_dir, clone2) = repo_clone(r1_dir).unwrap();

        let clone2_dir = clone2_dir.path().to_str().unwrap();

        // clone1

        let commit1 =
            write_commit_file(&clone1, "test.txt", "test", "commit1");

        sync::tag(clone1_dir, &commit1, "tag1").unwrap();

        push(clone1_dir, "origin", "master", false, None, None)
            .unwrap();
        push_tags(clone1_dir, "origin", None, None).unwrap();

        // clone2

        let _commit2 = write_commit_file(
            &clone2,
            "test2.txt",
            "test",
            "commit2",
        );

        assert_eq!(sync::get_tags(clone2_dir).unwrap().len(), 0);

        //lets fetch from origin
        let bytes =
            fetch_origin(clone2_dir, "master", None, None).unwrap();
        assert!(bytes > 0);

        sync::merge_upstream_commit(clone2_dir, "master").unwrap();

        assert_eq!(sync::get_tags(clone2_dir).unwrap().len(), 1);
    }
}
