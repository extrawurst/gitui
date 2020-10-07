//!

use super::commit::signature_allow_undefined_name;
use crate::{error::Error, error::Result, sync::utils};
use git2::{Oid, RebaseOptions};

/// This is the same as reword, but will abort and fix the repo if something goes wrong
pub fn reword_safe(
    repo_path: &str,
    commit_oid: Oid,
    message: &str,
) -> Result<()> {
    let repo = utils::repo(repo_path)?;
    let mut cur_branch_ref = None;
    // Find the head branch
    for b in repo.branches(None)? {
        let branch = b?.0;
        if branch.is_head() {
            cur_branch_ref = Some(String::from(
                branch
                    .get()
                    .name()
                    .expect("Branch name is not valid utf8"),
            ));
            break;
        }
    }

    match reword(repo_path, commit_oid, message) {
        Ok(()) => Ok(()),
        // Something went wrong, checkout the previous branch then error
        Err(e) => {
            if let Ok(mut rebase) = repo.open_rebase(None) {
                if let Some(cur_branch) = cur_branch_ref {
                    rebase.abort()?;
                    repo.set_head(&cur_branch)?;
                    repo.checkout_head(None)?;
                }
            }
            Err(e)
        }
    }
}

/// Changes the commit message of a commit with a specified oid
///
/// While this function is most commonly associated with doing a
/// reword opperation in an interactive rebase, that is not how it
/// is implimented in git2rs
///
/// This is dangerous if this errors, as the head will be detached so this should
/// always be wrapped by another function which aborts the rebase if something goes worng
pub fn reword(
    repo_path: &str,
    commit_oid: Oid,
    message: &str,
) -> Result<()> {
    let repo = utils::repo(repo_path)?;
    let sig = signature_allow_undefined_name(&repo)?;

    let parent_commit_oid = if let Ok(parent_commit) =
        repo.find_commit(commit_oid)?.parent(0)
    {
        Some(parent_commit.id())
    } else {
        None
    };

    let commit_to_change = if let Some(pc_oid) = parent_commit_oid {
        // Need to start at one previous to the commit, so
        // next point to the actual commit we want to change
        repo.find_annotated_commit(pc_oid)?
    } else {
        return Err(Error::NoParent);
        // Would the below work?
        // repo.find_annotated_commit(commit_oid)?
    };
    let mut top_branch_commit = None;
    let mut cur_branch_ref = None;
    let mut cur_branch_name = None;

    // Find the head branch
    for b in repo.branches(None)? {
        let branch = b?.0;
        if branch.is_head() {
            cur_branch_ref = Some(String::from(
                branch
                    .get()
                    .name()
                    .expect("Branch name is not valid utf8"),
            ));
            cur_branch_name = Some(String::from(
                branch
                    .name()?
                    .expect("Branch name is not valid utf8"),
            ));
            top_branch_commit = Some(repo.find_annotated_commit(
                branch.get().peel_to_commit()?.id(),
            )?);
            break;
        }
    }

    if let Some(top_branch_commit) = top_branch_commit {
        // Branch was found, so start a rebase
        let mut rebase = repo.rebase(
            Some(&top_branch_commit),
            Some(&commit_to_change),
            None,
            Some(&mut RebaseOptions::default()),
        )?;

        let mut target;

        rebase.next();
        if parent_commit_oid.is_none() {
            return Err(Error::NoParent);
        } else {
            target = rebase.commit(None, &sig, Some(message))?; //Some(message))?;
        }

        // Set target to top commit, don't know when the rebase will end
        // so have to loop till end
        while rebase.next().is_some() {
            target = rebase.commit(None, &sig, None)?;
        }
        rebase.finish(None)?;

        // Now override the current branch
        repo.branch(
            &cur_branch_name.expect("Couldn't unwrap branch name"),
            &repo.find_commit(target)?,
            true,
        )?;

        // Reset the head back to the branch then checkout head
        repo.set_head(
            &cur_branch_ref.expect("Couldn't unwrap branch name"),
        )?;
        repo.checkout_head(None)?;
    }
    Ok(())
}
