use super::{utils::repo, CommitId};
use crate::error::{Error, Result};
use git2::{Oid, Repository, StashFlags};
use scopetime::scope_time;

///
pub fn get_stashes(repo_path: &str) -> Result<Vec<Oid>> {
    scope_time!("get_stashes");

    let mut repo = repo(repo_path)?;

    let mut list = Vec::new();

    repo.stash_foreach(|_index, _msg, id| {
        list.push(*id);
        true
    })?;

    Ok(list)
}

///
pub fn stash_drop(repo_path: &str, stash_id: CommitId) -> Result<()> {
    scope_time!("stash_drop");

    let mut repo = repo(repo_path)?;

    let index = get_stash_index(&mut repo, stash_id.get_oid())?;

    repo.stash_drop(index)?;

    Ok(())
}

///
pub fn stash_apply(
    repo_path: &str,
    stash_id: CommitId,
) -> Result<()> {
    scope_time!("stash_apply");

    let mut repo = repo(repo_path)?;

    let index = get_stash_index(&mut repo, stash_id.get_oid())?;

    repo.stash_apply(index, None)?;

    Ok(())
}

fn get_stash_index(
    repo: &mut Repository,
    stash_id: Oid,
) -> Result<usize> {
    let mut idx = None;

    repo.stash_foreach(|index, _msg, id| {
        if *id == stash_id {
            idx = Some(index);
            false
        } else {
            true
        }
    })?;

    idx.ok_or_else(|| {
        Error::Generic("stash commit not found".to_string())
    })
}

///
pub fn stash_save(
    repo_path: &str,
    message: Option<&str>,
    include_untracked: bool,
    keep_index: bool,
) -> Result<()> {
    scope_time!("stash_save");

    let mut repo = repo(repo_path)?;

    let sig = repo.signature()?;

    let mut options = StashFlags::DEFAULT;

    if include_untracked {
        options.insert(StashFlags::INCLUDE_UNTRACKED);
    }
    if keep_index {
        options.insert(StashFlags::KEEP_INDEX)
    }

    repo.stash_save2(&sig, message, Some(options))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::{
        get_commits_info,
        tests::{get_statuses, repo_init},
    };
    use std::{fs::File, io::Write};

    #[test]
    fn test_smoke() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        assert_eq!(
            stash_save(repo_path, None, true, false).is_ok(),
            false
        );

        assert_eq!(get_stashes(repo_path).unwrap().is_empty(), true);
    }

    #[test]
    fn test_stashing() -> Result<()> {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join("foo.txt"))?
            .write_all(b"test\nfoo")?;

        assert_eq!(get_statuses(repo_path), (1, 0));

        stash_save(repo_path, None, true, false)?;

        assert_eq!(get_statuses(repo_path), (0, 0));

        Ok(())
    }

    #[test]
    fn test_stashes() -> Result<()> {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join("foo.txt"))?
            .write_all(b"test\nfoo")?;

        stash_save(repo_path, Some("foo"), true, false)?;

        let res = get_stashes(repo_path)?;

        assert_eq!(res.len(), 1);

        let infos =
            get_commits_info(repo_path, &[res[0]], 100).unwrap();

        assert_eq!(infos[0].message, "On master: foo");

        Ok(())
    }

    #[test]
    fn test_stash_nothing_untracked() -> Result<()> {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join("foo.txt"))?
            .write_all(b"test\nfoo")?;

        assert!(
            stash_save(repo_path, Some("foo"), false, false).is_err()
        );

        Ok(())
    }
}
