use std::path::PathBuf;

use git2::SubmoduleUpdateOptions;

use super::{repo, CommitId, RepoPath};
use crate::error::Result;

pub struct SubmoduleInfo {
	pub path: PathBuf,
	pub url: Option<String>,
	pub id: Option<CommitId>,
	pub head_id: Option<CommitId>,
}

///
pub fn get_submodules(
	repo_path: &RepoPath,
) -> Result<Vec<SubmoduleInfo>> {
	let repo = repo(repo_path)?;

	let res = repo
		.submodules()?
		.iter()
		.map(|s| SubmoduleInfo {
			path: s.path().to_path_buf(),
			id: s.workdir_id().map(CommitId::from),
			head_id: s.head_id().map(CommitId::from),
			url: s.url().map(String::from),
		})
		.collect();

	Ok(res)
}

///
pub fn update_submodule(
	repo_path: &RepoPath,
	path: &str,
) -> Result<()> {
	let repo = repo(repo_path)?;

	let mut submodule = repo.find_submodule(path)?;

	let mut options = SubmoduleUpdateOptions::new();

	submodule.update(true, Some(&mut options))?;

	Ok(())
}
