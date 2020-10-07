//!

use super::commit::signature_allow_undefined_name;
use super::CommitId;
use crate::{error::Result, sync::utils};
use crossbeam_channel::Sender;
use git2::{
    Cred, Error as GitError, FetchOptions, Oid, PackBuilderStage,
    PushOptions, RebaseOptions, RemoteCallbacks,
};
use scopetime::scope_time;

struct Rebase {}

impl Rebase {}

/// Changes the commit message of a commit with a specified hash
/// change_commit_message
///
/// While this function is most commonly associated with doing a
/// reword opperation in an interactive rebase, that is not how it
/// is implimented in git2rs
pub fn reword(
    repo_path: &str,
    commit_oid: Oid,
    message: &str,
) -> Result<()> {
    let repo = utils::repo(repo_path)?;
    let sig = signature_allow_undefined_name(&repo)?;
    let head = repo
        .find_annotated_commit(utils::get_head(repo_path)?.into())?;
    //let mut parent_commit_oid = None;
    let mut parent_commit_oid = None;

    if let Ok(parent_commit) = repo.find_commit(commit_oid)?.parent(0)
    {
        parent_commit_oid = Some(parent_commit.id());
    } else {
        parent_commit_oid = None;
    }
    //let parent_commit_oid =
    //  .unwrap().id();

    //let cur_commit = repo
    //  .find_annotated_commit(commit_oid)
    //.expect("Unable to find commit");
    // panic!("{:?}", cur_commit.refname());

    /* let new_commit_oid = c
            .amend(
                /*cur_commit.refname()*/
                Some("HEAD"), //&commit_oid.to_string()),
                None,
                None,
                None,
                Some(message),
                None,
            )
            .unwrap();
    */
    // panic!("{:?}", c);
    // Then begin a rebase
    let commit_to_change = if parent_commit_oid.is_some() {
        repo.find_annotated_commit(parent_commit_oid.unwrap())? //commit_oid)?;
    } else {
        repo.find_annotated_commit(commit_oid)?
    };
    let mut top_branch_commit = None;
    let mut cur_branch_ref = None;
    let mut cur_branch_name = None;
    for b in repo.branches(None).unwrap() {
        let branch = b?.0;
        if branch.is_head() {
            //cur_branch_ref
            cur_branch_ref =
                Some(String::from(branch.get().name().unwrap()));
            cur_branch_name =
                Some(String::from(branch.name().unwrap().unwrap()));
            // panic!("{:?}", branch.name());
            top_branch_commit = Some(repo.find_annotated_commit(
                branch.get().peel_to_commit()?.id(),
            )?);
            break;
        }

        //.iter().map(|(b, bt)| b.0.ishead()); //(commit_oid)?;
    }
    if let Some(top_branch_commit) = top_branch_commit {
        let mut rebase = repo
            .rebase(
                Some(&top_branch_commit),
                Some(&commit_to_change),
                None,
                Some(&mut RebaseOptions::default()),
            )
            .unwrap();

        //panic!("{:?}", rebase.operation_current());
        // Go to the first (and only) item
        let mut target;
        let cur_commit = rebase.next();
        if parent_commit_oid.is_none() {
            //repo.set_head(refname: &str)

            // There is no parent
            // so immediatly ammend
            repo.find_commit(cur_commit.unwrap().unwrap().id())
                .unwrap()
                .amend(
                    Some("rebase-merge-todo"),
                    None,
                    None,
                    None,
                    Some(message),
                    None,
                )
                .unwrap();
            target = rebase.commit(None, &sig, None)?;
        } else {
            target = rebase.commit(None, &sig, Some(message))?; //Some(message))?;
        }
        for item in rebase.next() {
            target = rebase.commit(None, &sig, None)?;
        }
        rebase.finish(None).unwrap();

        // Now override the current branch
        repo.branch(
            &cur_branch_name.unwrap(),
            &repo.find_commit(target).unwrap(),
            true,
        );

        repo.set_head(&cur_branch_ref.unwrap());
        repo.checkout_head(None);
        // Now reset master to the commit, which is now detached
        //repo.set_head(&(cur_branch_ref.unwrap()));

        //return Ok(());
    }
    Ok(())
    //cur_branch.
    //Some(&head)
}
