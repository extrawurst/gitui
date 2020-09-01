//!

use crate::{
    error::{Error, Result},
    sync::utils,
};
use scopetime::scope_time;
use utils::get_head_repo;

/// returns the branch-name head is currently pointing to
/// this might be expensive, see `cached::BranchName`
pub(crate) fn get_branch_name(repo_path: &str) -> Result<String> {
    scope_time!("get_branch_name");

    let repo = utils::repo(repo_path)?;

    let iter = repo.branches(None)?;

    for b in iter {
        let b = b?;

        if b.0.is_head() {
            let name = b.0.name()?.unwrap_or("");
            return Ok(name.into());
        }
    }

    Err(Error::NoHead)
}

/// creates a new branch pointing to current HEAD commit and updating HEAD to new branch
pub fn create_branch(repo_path: &str, name: &str) -> Result<()> {
    scope_time!("create_branch");

    let repo = utils::repo(repo_path)?;

    let head_id = get_head_repo(&repo)?;
    let head_commit = repo.find_commit(head_id.into())?;

    let branch = repo.branch(name, &head_commit, false)?;
    let branch_ref = branch.into_reference();
    let branch_ref_name =
        String::from_utf8(branch_ref.name_bytes().to_vec())?;
    repo.set_head(branch_ref_name.as_str())?;

    Ok(())
}

#[cfg(test)]
mod tests_branch_name {
    use super::*;
    use crate::sync::tests::{repo_init, repo_init_empty};

    #[test]
    fn test_smoke() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        assert_eq!(
            get_branch_name(repo_path).unwrap().as_str(),
            "master"
        );
    }

    #[test]
    fn test_empty_repo() {
        let (_td, repo) = repo_init_empty().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        assert!(matches!(
            get_branch_name(repo_path),
            Err(Error::NoHead)
        ));
    }
}

#[cfg(test)]
mod tests_create_branch {
    use super::*;
    use crate::sync::tests::repo_init;

    #[test]
    fn test_smoke() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        create_branch(repo_path, "branch1").unwrap();

        assert_eq!(
            get_branch_name(repo_path).unwrap().as_str(),
            "branch1"
        );
    }
}
