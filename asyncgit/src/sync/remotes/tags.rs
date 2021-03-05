//!

use super::utils;
use crate::error::Result;
use scopetime::scope_time;

///
pub(crate) fn push_tags(repo_path: &str, remote: &str) -> Result<()> {
    scope_time!("push_tags");

    let repo = utils::repo(repo_path)?;
    let mut remote = repo.find_remote(remote)?;

    repo.tag_foreach(|_id, name| {
        if let Ok(name) = String::from_utf8(name.into()) {
            remote.push(&[name], None).is_ok()
        } else {
            true
        }
    })?;

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
        push_tags(clone1_dir, "origin").unwrap();

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
