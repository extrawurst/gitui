use crate::{
    error::{Error, Result},
    sync::{commit, reset_stage, reset_workdir, utils, CommitId},
};
use git2::{BranchType, MergeOptions};
use scopetime::scope_time;

///
pub fn mergehead_ids(repo_path: &str) -> Result<Vec<CommitId>> {
    scope_time!("mergehead_ids");

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

    let annotated =
        repo.reference_to_annotated_commit(&branch.into_reference())?;

    let (analysis, _) = repo.merge_analysis(&[&annotated])?;

    //TODO: support merge on unborn
    if analysis.is_unborn() {
        return Err(Error::Generic("head is unborn".into()));
    }

    let mut opt = MergeOptions::default();

    repo.merge(&[&annotated], Some(&mut opt), None)?;

    Ok(())
}

///
pub fn merge_msg(_repo_path: &str) -> Result<String> {
    scope_time!("merge_msg");

    Ok(String::from("todo"))
}

///
pub fn merge_commit(
    repo_path: &str,
    msg: &str,
    _ids: &[CommitId],
) -> Result<CommitId> {
    scope_time!("merge_commit");

    commit(repo_path, msg)
    // let repo = utils::repo(repo_path)?;

    // let branch = repo.find_branch(branch, BranchType::Local)?;

    // let annotated =
    //     repo.reference_to_annotated_commit(&branch.into_reference())?;

    // let (analysis, _) = repo.merge_analysis(&[&annotated])?;

    // //TODO: support merge on unborn
    // if analysis.is_unborn() {
    //     return Err(Error::Generic("head is unborn".into()));
    // }

    // let mut opt = MergeOptions::default();

    // repo.merge(&[&annotated], Some(&mut opt), None)?;
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

        let mergeheads = mergehead_ids(repo_path).unwrap();

        assert_eq!(mergeheads[0], c1);
    }
}
