//! merging from upstream

use super::BranchType;
use crate::{error::Result, sync::utils};
use scopetime::scope_time;

///
pub fn branch_merge_upstream(
    repo_path: &str,
    branch: &str,
) -> Result<()> {
    scope_time!("branch_merge_upstream");

    let repo = utils::repo(repo_path)?;

    let branch = repo.find_branch(branch, BranchType::Local)?;
    let upstream = branch.upstream()?;

    let branch_commit = branch.into_reference().peel_to_commit()?;
    let upstream_commit =
        upstream.into_reference().peel_to_commit()?;

    let mut index =
        repo.merge_commits(&branch_commit, &upstream_commit, None)?;

    index.write()?;

    Ok(())
}
