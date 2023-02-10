use crate::error::Result;
use crate::sync::repository::repo;
use git2::{WorktreeLockStatus, WorktreePruneOptions};
use scopetime::scope_time;
use std::path::{Path, PathBuf};

use super::RepoPath;

/// Represents a worktree
#[derive(Debug)]
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
	/// Is worktree the current worktree
	pub is_current: bool,
}

/// Get all worktrees
pub fn worktrees(repo_path: &RepoPath) -> Result<Vec<WorkTree>> {
	scope_time!("worktrees");

	let repo_obj = repo(repo_path)?;

	Ok(repo_obj
		.worktrees()?
		.iter()
		.filter_map(|s| {
			if s.is_none() {
				log::error!("Error getting worktree: {:?}", s);
			};
			s
		})
		.map(|s| {
			let wt = repo_obj.find_worktree(s)?;
			Ok(WorkTree {
				name: s.to_string(),
				// branch: worktree_branch(s.unwrap(), &repo_obj).unwrap(),
				is_valid: wt.validate().is_ok(),
				path: wt.path().to_path_buf(),
				is_locked: match wt.is_locked()? {
					WorktreeLockStatus::Unlocked => false,
					WorktreeLockStatus::Locked(_) => true,
				},
				is_prunable: wt.is_prunable(None)?,
				is_current: wt.path() == repo_path.gitpath(),
			})
		})
		.filter_map(|s: Result<WorkTree>| {
			if s.is_err() {
				log::error!("Error getting worktree: {:?}", s);
			}
			s.ok()
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

/// Create worktree
pub fn create_worktree(
	repo_path: &RepoPath,
	name: &str,
) -> Result<()> {
	scope_time!("create_worktree");

	let repo_obj = repo(repo_path)?;
	let path_str = repo_path.gitpath().to_str().unwrap();

	// if we are in a worktree assume we want to create a worktree in the parent directory
	// This is not always accurate but it should work in most cases
	let real_path = match repo_obj.is_worktree() {
		true => format!("{}/../{}", path_str, &name),
		false => format!("{}{}", path_str, &name),
	};

	log::trace!("creating worktree in {:?}", real_path);
	repo_obj.worktree(name, &Path::new(&real_path), None)?;

	Ok(())
}

/// Prune a worktree
pub fn prune_worktree(
	repo_path: &RepoPath,
	name: &str,
	force: bool,
) -> Result<()> {
	scope_time!("prune_worktree");

	let repo_obj = repo(repo_path)?;

	let wt = repo_obj.find_worktree(name)?;
	wt.is_prunable(None)?;

	wt.prune(Some(
		WorktreePruneOptions::new()
			.valid(force)
			.locked(force)
			.working_tree(force),
	))?;

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

	// fails to create is branch already exists
	wt.validate()?;

	match wt.is_locked().unwrap() {
		WorktreeLockStatus::Unlocked => wt.lock(None)?,
		WorktreeLockStatus::Locked(_) => wt.unlock()?,
	}

	Ok(())
}
