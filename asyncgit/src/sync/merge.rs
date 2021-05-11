use crate::{
    error::{Error, Result},
    sync::{reset_stage, reset_workdir, utils, CommitId},
};
use git2::{BranchType, MergeOptions};
use scopetime::scope_time;

///
pub fn merge_state_info(repo_path: &str) -> Result<Vec<CommitId>> {
    scope_time!("merge_state_info");

    let mut repo = utils::repo(repo_path)?;

    let mut ids: Vec<CommitId> = Vec::new();
    repo.mergehead_foreach(|id| {
        ids.push(CommitId::from(*id));
        true
    })?;

    Ok(ids)
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::{
        create_branch,
        tests::{repo_init, write_commit_file},
    };

    #[test]
    fn test_smoke() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        let c1 =
            write_commit_file(&repo, "test.txt", "test", "commit1");

        create_branch(repo_path, "foo").unwrap();

        write_commit_file(&repo, "test.txt", "test2", "commit2");

        merge_branch(repo_path, "master").unwrap();

        let mergeheads = merge_state_info(repo_path).unwrap();

        assert_eq!(mergeheads[0], c1);
    }
}
