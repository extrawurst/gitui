use git2::{BranchType, Repository};
use scopetime::scope_time;

use crate::{
	error::{Error, Result},
	sync::utils,
};

use super::CommitId;

/// rebase current HEAD on `branch`
pub fn rebase_branch(
	repo_path: &str,
	branch: &str,
) -> Result<CommitId> {
	scope_time!("rebase_branch");

	let repo = utils::repo(repo_path)?;

	rebase_branch_repo(&repo, branch)
}

fn rebase_branch_repo(
	repo: &Repository,
	branch_name: &str,
) -> Result<CommitId> {
	let branch = repo.find_branch(branch_name, BranchType::Local)?;

	let annotated =
		repo.reference_to_annotated_commit(&branch.into_reference())?;

	conflict_free_rebase(repo, &annotated)
}

/// rebase attempt which aborts and undo's rebase if any conflict appears
pub fn conflict_free_rebase(
	repo: &git2::Repository,
	commit: &git2::AnnotatedCommit,
) -> Result<CommitId> {
	let mut rebase = repo.rebase(None, Some(commit), None, None)?;
	let signature =
		crate::sync::commit::signature_allow_undefined_name(repo)?;
	let mut last_commit = None;
	while let Some(op) = rebase.next() {
		let _op = op?;

		if repo.index()?.has_conflicts() {
			rebase.abort()?;
			return Err(Error::RebaseConflict);
		}

		let c = rebase.commit(None, &signature, None)?;

		last_commit = Some(CommitId::from(c));
	}

	if repo.index()?.has_conflicts() {
		rebase.abort()?;
		return Err(Error::RebaseConflict);
	}

	rebase.finish(Some(&signature))?;

	last_commit.ok_or_else(|| {
		Error::Generic(String::from("no commit rebased"))
	})
}

#[cfg(test)]
mod tests {
	use crate::sync::{
		checkout_branch, create_branch,
		rebase::rebase_branch,
		repo_state,
		tests::{repo_init, write_commit_file},
		CommitId, RepoState,
	};
	use git2::Repository;

	fn parent_ids(repo: &Repository, c: CommitId) -> Vec<CommitId> {
		let foo = repo
			.find_commit(c.into())
			.unwrap()
			.parent_ids()
			.map(|id| CommitId::from(id))
			.collect();

		foo
	}

	#[test]
	fn test_smoke() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path = root.as_os_str().to_str().unwrap();

		let c1 =
			write_commit_file(&repo, "test1.txt", "test", "commit1");

		create_branch(repo_path, "foo").unwrap();

		let c2 =
			write_commit_file(&repo, "test2.txt", "test", "commit2");

		assert_eq!(parent_ids(&repo, c2), vec![c1]);

		checkout_branch(repo_path, "refs/heads/master").unwrap();

		let c3 =
			write_commit_file(&repo, "test3.txt", "test", "commit3");

		checkout_branch(repo_path, "refs/heads/foo").unwrap();

		let r = rebase_branch(repo_path, "master").unwrap();

		assert_eq!(parent_ids(&repo, r), vec![c3]);
	}

	#[test]
	fn test_conflict() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path = root.as_os_str().to_str().unwrap();

		write_commit_file(&repo, "test.txt", "test1", "commit1");

		create_branch(repo_path, "foo").unwrap();

		write_commit_file(&repo, "test.txt", "test2", "commit2");

		checkout_branch(repo_path, "refs/heads/master").unwrap();

		write_commit_file(&repo, "test.txt", "test3", "commit3");

		checkout_branch(repo_path, "refs/heads/foo").unwrap();

		let res = rebase_branch(repo_path, "master");

		assert!(res.is_err());

		assert_eq!(repo_state(repo_path).unwrap(), RepoState::Clean);
	}
}
