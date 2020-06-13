//!

use crate::{
    error::{Error, Result},
    sync::utils,
};
use scopetime::scope_time;

/// returns the branch-name head is currently pointing to
pub fn get_branch_name(repo_path: &str) -> Result<String> {
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

#[cfg(test)]
mod tests {
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
