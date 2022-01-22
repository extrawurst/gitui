use super::{repo, utils::bytes2string, RepoPath};
use crate::error::Result;

///
pub fn get_submodules(repo_path: &RepoPath) -> Result<Vec<String>> {
	let repo = repo(repo_path)?;

	let submodules = repo.submodules()?;

	let mut modules = Vec::with_capacity(submodules.len());
	for s in submodules {
		modules.push(bytes2string(s.name_bytes())?);
	}

	Ok(modules)
}
