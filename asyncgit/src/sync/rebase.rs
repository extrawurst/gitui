//!

use super::commit::signature_allow_undefined_name;
use super::CommitId;
use crate::{error::Error, error::Result, sync::utils};
use crossbeam_channel::Sender;
use git2::{
    Cred, Error as GitError, FetchOptions, Oid, PackBuilderStage,
    PushOptions, RebaseOptions, RemoteCallbacks,
};
use scopetime::scope_time;

/// This is the same as reword, but will abort and fix the repo if something goes wrong
pub fn reword_safe(
    repo_path: &str,
    commit_oid: Oid,
    message: &str,
) -> Result<()> {
    let repo = utils::repo(repo_path)?;
    if reword(repo_path, commit_oid, message).is_ok() {
        Ok(())
    } else {
        // Something went wrong, checkout the master branch
        // then error
        if let Ok(mut rebase) = repo.open_rebase(None) {
            rebase.abort()?;
            repo.set_head("master")?;
            repo.checkout_head(None)?;
        }
        Err(Error::Rebase)
    }
}

/// Changes the commit message of a commit with a specified oid
///
/// While this function is most commonly associated with doing a
/// reword opperation in an interactive rebase, that is not how it
/// is implimented in git2rs
///
/// This is dangrous if this errors, as the head will be detached so this should
/// always be wrapped by another function which aborts the rebase and checks-out the
/// previous branch if something goes worng
pub fn reword(
    repo_path: &str,
    commit_oid: Oid,
    message: &str,
) -> Result<()> {
    let repo = utils::repo(repo_path)?;
    let sig = signature_allow_undefined_name(&repo)?;
    let head = repo
        .find_annotated_commit(utils::get_head(repo_path)?.into())?;

    let parent_commit_oid = if let Ok(parent_commit) =
        repo.find_commit(commit_oid)?.parent(0)
    {
        Some(parent_commit.id())
    } else {
        None
    };

    let commit_to_change = if parent_commit_oid.is_some() {
        // Need to start at one previous to the commit, so
        // next point to the actual commit we want to change
        repo.find_annotated_commit(parent_commit_oid.unwrap())?
    } else {
        return Err(Error::NoParent);
        // Would the below work?
        // repo.find_annotated_commit(commit_oid)?
    };
    let mut top_branch_commit = None;
    let mut cur_branch_ref = None;
    let mut cur_branch_name = None;

    // Find the head branch
    for b in repo.branches(None).unwrap() {
        let branch = b?.0;
        if branch.is_head() {
            cur_branch_ref =
                Some(String::from(branch.get().name().unwrap()));
            cur_branch_name =
                Some(String::from(branch.name().unwrap().unwrap()));
            top_branch_commit = Some(repo.find_annotated_commit(
                branch.get().peel_to_commit()?.id(),
            )?);
            break;
        }
    }

    if let Some(top_branch_commit) = top_branch_commit {
        // Branch was found, so start a rebase
        let mut rebase = repo
            .rebase(
                Some(&top_branch_commit),
                Some(&commit_to_change),
                None,
                Some(&mut RebaseOptions::default()),
            )
            .unwrap();

        let mut target;

        rebase.next().unwrap()?;
        if parent_commit_oid.is_none() {
            return Err(Error::NoParent);
        } else {
            target = rebase.commit(None, &sig, Some(message))?; //Some(message))?;
        }

        // Set target to top commit, don't know when the rebase will end
        // so have to loop till end
        for _ in rebase.next() {
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
