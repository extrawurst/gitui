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

///
pub struct BranchForDisplay {
    ///
    pub name: String,
    ///
    pub reference: String,
    ///
    pub top_commit_message: String,
    ///
    pub top_commit_reference: String,
    ///
    pub is_head: bool,
}

/// TODO make this cached
/// Used to return only the nessessary information for displaying a branch
/// rather than an iterator over the actual branches
pub fn get_branches_to_display(
    repo_path: &str,
) -> Vec<BranchForDisplay> {
    if let Ok(cur_repo) = utils::repo(repo_path) {
        cur_repo
            .branches(None)
            .map_err(super::super::error::Error::Git)
            .expect("")
            .map(|b| {
                let branch = &(&b).as_ref().expect("").0;

                let top_commit =
                    branch.get().peel_to_commit().expect("");

                let mut commit_id = top_commit.id().to_string();
                commit_id.truncate(7);
                BranchForDisplay {
                    name: match branch.name().expect("") {
                        Some(name) => String::from(name),
                        None => String::from(""),
                    },

                    reference: match branch.get().name() {
                        Some(name) => String::from(name),
                        None => String::from(""),
                    },

                    top_commit_message: match top_commit.message()//.shorthand()
                     {
                         Some(name) => String::from(name.trim_end()),
                         None => String::from(""),
                     },

                    top_commit_reference: commit_id,

                    is_head: branch.is_head(),
                }
            })
            .collect::<_>()
    } else {
        vec![]
    }
}

/// Modify HEAD to point to a branch
/// then checkout head
pub fn checkout_branch(
    repo_path: &str,
    branch_ref: &str,
) -> Result<()> {
    // This defaults to a safe checkout, so don't delete anything that
    // hasn't been committed or stashed, in this case it will Err
    if let Ok(repo) = utils::repo(repo_path) {
        if repo.set_head(branch_ref).is_ok()
            && repo.checkout_head(None).is_ok()
        {
            return Ok(());
        }
    }

    // TODO change this
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
