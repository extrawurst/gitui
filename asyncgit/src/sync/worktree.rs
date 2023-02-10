use crate::error::Result;
use crate::sync::repository::repo;
use git2::WorktreeLockStatus;
use scopetime::scope_time;
use std::path::{Path, PathBuf};

use super::RepoPath;

/// This should kinda represent a worktree
pub struct WorkTree {
	/// Worktree name (wich is also the folder i think)
	pub name: String,
	// Worktree branch name
	// pub branch: String,
	/// Is the worktree valid
	pub is_valid: bool,
	/// Worktree path
	pub path: PathBuf,
	/// Is worktree locked
	pub is_locked: bool,
	/// Can worktree be pruned
	pub is_prunable: bool,
}

/// Get all worktrees
pub fn worktrees(repo_path: &RepoPath) -> Result<Vec<WorkTree>> {
	scope_time!("worktrees");

	let repo_obj = repo(repo_path)?;

	Ok(repo_obj
		.worktrees()?
		.iter()
		.map(|s| {
			let wt = repo_obj.find_worktree(s.unwrap()).unwrap();
			WorkTree {
				name: s.unwrap().to_string(),
				// branch: worktree_branch(s.unwrap(), &repo_obj).unwrap(),
				is_valid: wt.validate().is_ok(),
				path: wt.path().to_path_buf(),
				is_locked: match wt.is_locked().unwrap() {
					WorktreeLockStatus::Unlocked => false,
					WorktreeLockStatus::Locked(_) => true,
				},
				is_prunable: wt.is_prunable(None).unwrap(),
			}
		})
		.collect())
}

/// Find a worktree path
pub fn find_worktree(
	repo_path: &RepoPath,
	name: &str,
) -> Result<RepoPath> {
	scope_time!("find_worktree");

	let repo_obj = repo(repo_path)?;

	let wt = repo_obj.find_worktree(name)?;
	wt.validate()?;

	Ok(RepoPath::Path(wt.path().to_path_buf()))
}

/// create worktree
/// NOTE: creates the folder for the worktree in current directory instead of the path
/// of the repo
pub fn create_worktree(
	repo_path: &RepoPath,
	name: &str,
) -> Result<()> {
	scope_time!("create_worktree");

	log::trace!("creating worktree in {:?}", repo_path);
	let repo_obj = repo(repo_path)?;

	repo_obj.worktree(name, &Path::new(&name), None)?;

	Ok(())
}

/// Prune a worktree
pub fn prune_worktree(
	repo_path: &RepoPath,
	name: &str,
) -> Result<()> {
	scope_time!("prune_worktree");

	let repo_obj = repo(repo_path)?;

	let wt = repo_obj.find_worktree(name)?;
	wt.is_prunable(None)?;
	wt.prune(None)?;

	Ok(())
}

/// Toggle lock on a worktree
pub fn toggle_worktree_lock(
	repo_path: &RepoPath,
	name: &str,
) -> Result<()> {
	scope_time!("toggle_lock_worktree");

	let repo_obj = repo(repo_path)?;

	let wt = repo_obj.find_worktree(name)?;
	wt.validate()?;

	match wt.is_locked().unwrap() {
		WorktreeLockStatus::Unlocked => wt.lock(None)?,
		WorktreeLockStatus::Locked(_) => wt.unlock()?,
	}

	Ok(())
}
