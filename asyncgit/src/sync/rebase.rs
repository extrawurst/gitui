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
            cur_branch_ref = Some(String::from_utf8(
                branch.get().name_bytes().to_vec(),
            )?);
            break;
        }
    }

    match reword(repo_path, commit_oid, message) {
        Ok(()) => Ok(()),
        // Something went wrong, checkout the previous branch then error
        Err(e) => {
            if let Ok(mut rebase) = repo.open_rebase(None) {
                match cur_branch_ref {
                    Some(cur_branch) => {
                        rebase.abort()?;
                        repo.set_head(&cur_branch)?;
                        repo.checkout_head(None)?;
                    }
                    None => return Err(Error::NoBranch),
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
fn reword(
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
        // first rebase.next() points to the actual commit we want to change
        repo.find_annotated_commit(pc_oid)?
    } else {
        return Err(Error::NoParent);
    };
    let mut top_branch_commit = None;
    let mut cur_branch_ref = None;
    let mut cur_branch_name = None;

    // Find the head branch
    for b in repo.branches(None)? {
        let branch = b?.0;
        if branch.is_head() {
            // When getting the branch name/ref, make sure both are valid utf8
            cur_branch_ref = Some(String::from_utf8(Vec::from(
                branch.get().name_bytes(),
            ))?);
            cur_branch_name = Some(String::from_utf8(Vec::from(
                branch.name_bytes()?,
            ))?);
            top_branch_commit = Some(repo.find_annotated_commit(
                branch.get().peel_to_commit()?.id(),
            )?);
            break;
        }
    }

    if let (
        Some(top_branch_commit),
        Some(cur_branch_name),
        Some(cur_branch_ref),
    ) = (top_branch_commit, cur_branch_name, cur_branch_ref)
    {
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
            target = rebase.commit(None, &sig, Some(message))?;
        }

        // Set target to top commit, don't know when the rebase will end
        // so have to loop till end
        while rebase.next().is_some() {
            target = rebase.commit(None, &sig, None)?;
        }
        rebase.finish(None)?;

        // Now override the previous branch
        repo.branch(
            &cur_branch_name,
            &repo.find_commit(target)?,
            true,
        )?;

        // Reset the head back to the branch then checkout head
        repo.set_head(&cur_branch_ref)?;
        repo.checkout_head(None)?;
        return Ok(());
    }
    // Repo is not on a branch, possibly detached head
    Err(Error::NoBranch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::{
        commit, stage_add_file, tests::repo_init_empty,
    };
    use std::{fs::File, io::Write, path::Path};

    #[test]
    fn test_reword() -> Result<()> {
        let file_path = Path::new("foo");
        let (_td, repo) = repo_init_empty().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))?.write_all(b"a")?;
        stage_add_file(repo_path, file_path).unwrap();
        commit(repo_path, "commit1").unwrap();
        File::create(&root.join(file_path))?.write_all(b"ab")?;
        stage_add_file(repo_path, file_path).unwrap();
        let oid2 = commit(repo_path, "commit2").unwrap();

        let branch =
            repo.branches(None).unwrap().next().unwrap().unwrap().0;
        let branch_ref = branch.get();
        let commit_ref = branch_ref.peel_to_commit().unwrap();
        let message = commit_ref.message().unwrap();

        assert_eq!(message, "commit2");

        reword_safe(repo_path, oid2.into(), "NewCommitMessage")
            .unwrap();

        // Need to get the branch again as top oid has changed
        let branch =
            repo.branches(None).unwrap().next().unwrap().unwrap().0;
        let branch_ref = branch.get();
        let commit_ref_new = branch_ref.peel_to_commit().unwrap();
        let message_new = commit_ref_new.message().unwrap();
        assert_eq!(message_new, "NewCommitMessage");

        Ok(())
    }
}
