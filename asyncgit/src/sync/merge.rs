use crate::{
    error::{Error, Result},
    sync::{reset_stage, reset_workdir, utils},
};
use git2::{BranchType, MergeOptions};
use scopetime::scope_time;

/// does these steps:
/// * reset all staged changes,
/// * revert all changes in workdir
/// * cleanup repo merge state
pub fn abort_merge(repo_path: &str) -> Result<()> {
    scope_time!("cleanup_state");

    let repo = utils::repo(repo_path)?;

    reset_stage(repo_path, "*")?;
    reset_workdir(repo_path, "*")?;

    repo.cleanup_state()?;

    Ok(())
}

///
pub fn merge_branch(repo_path: &str, branch: &str) -> Result<()> {
    scope_time!("merge_branch");

    let repo = utils::repo(repo_path)?;

    let branch = repo.find_branch(branch, BranchType::Local)?;

    let id = branch.into_reference().peel_to_commit()?;

    let annotated = repo.find_annotated_commit(id.id())?;

    let (analysis, _) = repo.merge_analysis(&[&annotated])?;

    //TODO: support merge on unborn
    if analysis.is_unborn() {
        return Err(Error::Generic("head is unborn".into()));
    }

    let mut opt = MergeOptions::default();

    repo.merge(&[&annotated], Some(&mut opt), None)?;

    Ok(())
}
