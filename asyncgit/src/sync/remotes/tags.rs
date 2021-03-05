//!

use std::collections::HashSet;

use super::{push::remote_callbacks, utils};
use crate::{error::Result, sync::cred::BasicAuthCredential};
use crossbeam_channel::Sender;
use git2::{Direction, PushOptions};
use scopetime::scope_time;

pub(crate) struct PushTagsProgress {
    pub pushed: usize,
    pub total: usize,
}

/// lists the remotes tags
fn remote_tag_refs(
    repo_path: &str,
    remote: &str,
    basic_credential: Option<BasicAuthCredential>,
) -> Result<Vec<String>> {
    scope_time!("remote_tags");

    let repo = utils::repo(repo_path)?;
    let mut remote = repo.find_remote(remote)?;
    let conn = remote.connect_auth(
        Direction::Fetch,
        Some(remote_callbacks(None, basic_credential)),
        None,
    )?;

    let remote_heads = conn.list()?;
    let remote_tags = remote_heads
        .iter()
        .map(|s| s.name().to_string())
        .filter(|name| {
            name.starts_with("refs/tags/") && !name.ends_with("^{}")
        })
        .collect::<Vec<_>>();

    Ok(remote_tags)
}

/// lists the remotes tags missing
fn tags_missing_remote(
    repo_path: &str,
    remote: &str,
    basic_credential: Option<BasicAuthCredential>,
) -> Result<Vec<String>> {
    scope_time!("tags_missing_remote");

    let repo = utils::repo(repo_path)?;
    let tags = repo.tag_names(None)?;

    let mut local_tags = tags
        .iter()
        .filter_map(|tag| tag.map(|tag| format!("refs/tags/{}", tag)))
        .collect::<HashSet<_>>();
    let remote_tags =
        remote_tag_refs(repo_path, remote, basic_credential)?;

    for t in remote_tags {
        local_tags.remove(&t);
    }

    Ok(local_tags.into_iter().collect())
}

///
pub(crate) fn push_tags(
    repo_path: &str,
    remote: &str,
    basic_credential: Option<BasicAuthCredential>,
    progress_sender: Option<Sender<PushTagsProgress>>,
) -> Result<()> {
    scope_time!("push_tags");

    let tags_missing = tags_missing_remote(
        repo_path,
        remote,
        basic_credential.clone(),
    )?;

    let repo = utils::repo(repo_path)?;
    let mut remote = repo.find_remote(remote)?;

    let total = tags_missing.len();

    for (idx, tag) in tags_missing.into_iter().enumerate() {
        let mut options = PushOptions::new();
        options.remote_callbacks(remote_callbacks(
            None,
            basic_credential.clone(),
        ));
        options.packbuilder_parallelism(0);
        remote.push(&[tag.as_str()], Some(&mut options))?;

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

    #[test]
    fn test_get_remote_tags() {
        let (r1_dir, _repo) = repo_init_bare().unwrap();
        let r1_dir = r1_dir.path().to_str().unwrap();

        let (clone1_dir, clone1) = repo_clone(r1_dir).unwrap();

        let clone1_dir = clone1_dir.path().to_str().unwrap();

        let (clone2_dir, _clone2) = repo_clone(r1_dir).unwrap();

        let clone2_dir = clone2_dir.path().to_str().unwrap();

        // clone1

        let commit1 =
            write_commit_file(&clone1, "test.txt", "test", "commit1");

        sync::tag(clone1_dir, &commit1, "tag1").unwrap();

        push(clone1_dir, "origin", "master", false, None, None)
            .unwrap();
        push_tags(clone1_dir, "origin", None, None).unwrap();

        // clone2

        let tags =
            remote_tag_refs(clone2_dir, "origin", None).unwrap();

        assert_eq!(
            tags.as_slice(),
            &[String::from("refs/tags/tag1")]
        );
    }

    #[test]
    fn test_tags_missing_remote() {
        let (r1_dir, _repo) = repo_init_bare().unwrap();
        let r1_dir = r1_dir.path().to_str().unwrap();

        let (clone1_dir, clone1) = repo_clone(r1_dir).unwrap();

        let clone1_dir = clone1_dir.path().to_str().unwrap();

        // clone1

        let commit1 =
            write_commit_file(&clone1, "test.txt", "test", "commit1");

        sync::tag(clone1_dir, &commit1, "tag1").unwrap();

        push(clone1_dir, "origin", "master", false, None, None)
            .unwrap();

        let tags_missing =
            tags_missing_remote(clone1_dir, "origin", None).unwrap();

        assert_eq!(
            tags_missing.as_slice(),
            &[String::from("refs/tags/tag1")]
        );
        push_tags(clone1_dir, "origin", None, None).unwrap();
        let tags_missing =
            tags_missing_remote(clone1_dir, "origin", None).unwrap();
        assert!(tags_missing.is_empty());
    }
}
