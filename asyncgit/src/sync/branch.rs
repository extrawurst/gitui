//!

use super::{remotes::get_first_remote_in_repo, utils::bytes2string};
use crate::{
    error::{Error, Result},
    sync::{utils, CommitId},
};
use git2::{BranchType, Repository};
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

///
pub struct BranchForDisplay {
    ///
    pub name: String,
    ///
    pub reference: String,
    ///
    pub top_commit_message: String,
    ///
    pub top_commit: CommitId,
    ///
    pub is_head: bool,
    ///
    pub has_upstream: bool,
}

/// Used to return only the nessessary information for displaying a branch
/// rather than an iterator over the actual branches
pub fn get_branches_to_display(
    repo_path: &str,
) -> Result<Vec<BranchForDisplay>> {
    scope_time!("get_branches_to_display");

    let cur_repo = utils::repo(repo_path)?;
    let branches_for_display = cur_repo
        .branches(Some(BranchType::Local))?
        .map(|b| {
            let branch = b?.0;
            let top_commit = branch.get().peel_to_commit()?;

            Ok(BranchForDisplay {
                name: bytes2string(branch.name_bytes()?)?,
                reference: bytes2string(branch.get().name_bytes())?,
                top_commit_message: bytes2string(
                    top_commit.summary_bytes().unwrap_or_default(),
                )?,
                top_commit: top_commit.id().into(),
                is_head: branch.is_head(),
                has_upstream: branch.upstream().is_ok(),
            })
        })
        .filter_map(Result::ok)
        .collect();

    Ok(branches_for_display)
}

///
#[derive(Debug, Default)]
pub struct BranchCompare {
    ///
    pub ahead: usize,
    ///
    pub behind: usize,
}

///
pub(crate) fn branch_set_upstream(
    repo: &Repository,
    branch_name: &str,
) -> Result<()> {
    scope_time!("branch_set_upstream");

    let mut branch =
        repo.find_branch(branch_name, BranchType::Local)?;

    if branch.upstream().is_err() {
        let remote = get_first_remote_in_repo(repo)?;
        let upstream_name = format!("{}/{}", remote, branch_name);
        branch.set_upstream(Some(upstream_name.as_str()))?;
    }

    Ok(())
}

///
pub fn branch_compare_upstream(
    repo_path: &str,
    branch: &str,
) -> Result<BranchCompare> {
    scope_time!("branch_compare_upstream");

    let repo = utils::repo(repo_path)?;

    let branch = repo.find_branch(branch, BranchType::Local)?;

    let upstream = branch.upstream()?;

    let branch_commit =
        branch.into_reference().peel_to_commit()?.id();

    let upstream_commit =
        upstream.into_reference().peel_to_commit()?.id();

    let (ahead, behind) =
        repo.graph_ahead_behind(branch_commit, upstream_commit)?;

    Ok(BranchCompare { ahead, behind })
}

/// Modify HEAD to point to a branch then checkout head, does not work if there are uncommitted changes
pub fn checkout_branch(
    repo_path: &str,
    branch_ref: &str,
) -> Result<()> {
    scope_time!("checkout_branch");

    // This defaults to a safe checkout, so don't delete anything that
    // hasn't been committed or stashed, in this case it will Err
    let repo = utils::repo(repo_path)?;
    let cur_ref = repo.head()?;
    let statuses = repo.statuses(Some(
        git2::StatusOptions::new().include_ignored(false),
    ))?;

    if statuses.is_empty() {
        repo.set_head(branch_ref)?;

        if let Err(e) = repo.checkout_head(Some(
            git2::build::CheckoutBuilder::new().force(),
        )) {
            // This is safe beacuse cur_ref was just found
            repo.set_head(
                bytes2string(cur_ref.name_bytes())?.as_str(),
            )?;
            return Err(Error::Git(e));
        }
        Ok(())
    } else {
        Err(Error::Generic(
            format!("Cannot change branch. There are unstaged/staged changes which have not been committed/stashed. There is {:?} changes preventing checking out a different branch.",  statuses.len()),
        ))
    }
}

/// The user must not be on the branch for the branch to be deleted
pub fn delete_branch(
    repo_path: &str,
    branch_ref: &str,
) -> Result<()> {
    scope_time!("delete_branch");

    let repo = utils::repo(repo_path)?;
    let branch_as_ref = repo.find_reference(branch_ref)?;
    let mut branch = git2::Branch::wrap(branch_as_ref);
    if !branch.is_head() {
        branch.delete()?;
    } else {
        return Err(Error::Generic("You cannot be on the branch you want to delete, switch branch, then delete this branch".to_string()));
    }
    Ok(())
}

/// Rename the branch reference
pub fn rename_branch(
    repo_path: &str,
    branch_ref: &str,
    new_name: &str,
) -> Result<()> {
    scope_time!("delete_branch");

    let repo = utils::repo(repo_path)?;
    let branch_as_ref = repo.find_reference(branch_ref)?;
    let mut branch = git2::Branch::wrap(branch_as_ref);
    branch.rename(new_name, true)?;

    Ok(())
}

/// creates a new branch pointing to current HEAD commit and updating HEAD to new branch
pub fn create_branch(repo_path: &str, name: &str) -> Result<()> {
    scope_time!("create_branch");

    let repo = utils::repo(repo_path)?;

    let head_id = get_head_repo(&repo)?;
    let head_commit = repo.find_commit(head_id.into())?;

    let branch = repo.branch(name, &head_commit, false)?;
    let branch_ref = branch.into_reference();
    let branch_ref_name = bytes2string(branch_ref.name_bytes())?;
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

#[cfg(test)]
mod tests_branch_compare {
    use super::*;
    use crate::sync::tests::repo_init;

    #[test]
    fn test_smoke() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        create_branch(repo_path, "test").unwrap();

        let res = branch_compare_upstream(repo_path, "test");

        assert_eq!(res.is_err(), true);
    }
}

#[cfg(test)]
mod tests_branches {
    use super::*;
    use crate::sync::tests::repo_init;

    #[test]
    fn test_smoke() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        assert_eq!(
            get_branches_to_display(repo_path)
                .unwrap()
                .iter()
                .map(|b| b.name.clone())
                .collect::<Vec<_>>(),
            vec!["master"]
        );
    }

    #[test]
    fn test_multiple() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        create_branch(repo_path, "test").unwrap();

        assert_eq!(
            get_branches_to_display(repo_path)
                .unwrap()
                .iter()
                .map(|b| b.name.clone())
                .collect::<Vec<_>>(),
            vec!["master", "test"]
        );
    }
}

#[cfg(test)]
mod tests_checkout {
    use super::*;
    use crate::sync::tests::repo_init;

    #[test]
    fn test_smoke() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        assert!(
            checkout_branch(repo_path, "refs/heads/master").is_ok()
        );
        assert!(
            checkout_branch(repo_path, "refs/heads/foobar").is_err()
        );
    }

    #[test]
    fn test_multiple() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        create_branch(repo_path, "test").unwrap();

        assert!(checkout_branch(repo_path, "refs/heads/test").is_ok());
        assert!(
            checkout_branch(repo_path, "refs/heads/master").is_ok()
        );
        assert!(checkout_branch(repo_path, "refs/heads/test").is_ok());
    }
}

#[cfg(test)]
mod test_delete_branch {
    use super::*;
    use crate::sync::tests::repo_init;

    #[test]
    fn test_delete_branch() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        create_branch(repo_path, "branch1").unwrap();
        create_branch(repo_path, "branch2").unwrap();

        checkout_branch(repo_path, "refs/heads/branch1").unwrap();

        assert_eq!(
            repo.branches(None)
                .unwrap()
                .nth(1)
                .unwrap()
                .unwrap()
                .0
                .name()
                .unwrap()
                .unwrap(),
            "branch2"
        );

        delete_branch(repo_path, "refs/heads/branch2").unwrap();

        assert_eq!(
            repo.branches(None)
                .unwrap()
                .nth(1)
                .unwrap()
                .unwrap()
                .0
                .name()
                .unwrap()
                .unwrap(),
            "master"
        );
    }
}

#[cfg(test)]
mod test_rename_branch {
    use super::*;
    use crate::sync::tests::repo_init;

    #[test]
    fn test_rename_branch() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        create_branch(repo_path, "branch1").unwrap();

        checkout_branch(repo_path, "refs/heads/branch1").unwrap();

        assert_eq!(
            repo.branches(None)
                .unwrap()
                .nth(0)
                .unwrap()
                .unwrap()
                .0
                .name()
                .unwrap()
                .unwrap(),
            "branch1"
        );

        rename_branch(repo_path, "refs/heads/branch1", "AnotherName")
            .unwrap();

        assert_eq!(
            repo.branches(None)
                .unwrap()
                .nth(0)
                .unwrap()
                .unwrap()
                .0
                .name()
                .unwrap()
                .unwrap(),
            "AnotherName"
        );
    }
}
