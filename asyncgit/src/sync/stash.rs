use super::utils::repo;
use crate::error::Result;
use git2::{Oid, StashFlags};
use scopetime::scope_time;

///
#[allow(dead_code)]
pub struct StashItem {
    pub msg: String,
    index: usize,
    id: Oid,
}

///
#[allow(dead_code)]
pub struct StashItems(Vec<StashItem>);

///
#[allow(dead_code)]
pub fn get_stashes(repo_path: &str) -> Result<StashItems> {
    scope_time!("get_stashes");

    let mut repo = repo(repo_path)?;

    let mut list = Vec::new();

    repo.stash_foreach(|index, msg, id| {
        list.push(StashItem {
            msg: msg.to_string(),
            index,
            id: *id,
        });
        true
    })?;

    Ok(StashItems(list))
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
    use crate::sync::tests::{get_statuses, repo_init};
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

        assert_eq!(
            get_stashes(repo_path).unwrap().0.is_empty(),
            true
        );
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

        assert_eq!(res.0.len(), 1);
        assert_eq!(res.0[0].msg, "On master: foo");
        assert_eq!(res.0[0].index, 0);

        Ok(())
    }
}
