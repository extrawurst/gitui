use std::path::PathBuf;

use git2::SubmoduleUpdateOptions;
use scopetime::scope_time;

use super::{repo, CommitId, RepoPath};
use crate::{error::Result, Error};

pub use git2::SubmoduleStatus;

///
pub struct SubmoduleInfo {
	///
	pub path: PathBuf,
	///
	pub url: Option<String>,
	///
	pub id: Option<CommitId>,
	///
	pub head_id: Option<CommitId>,
	///
	pub status: SubmoduleStatus,
}

impl SubmoduleInfo {
	///
	pub fn get_repo_path(
		&self,
		repo_path: &RepoPath,
	) -> Result<RepoPath> {
		let repo = repo(repo_path)?;
		let wd = repo.workdir().ok_or(Error::NoWorkDir)?;

		Ok(RepoPath::Path(wd.join(self.path.clone())))
	}
}

///
pub fn get_submodules(
	repo_path: &RepoPath,
) -> Result<Vec<SubmoduleInfo>> {
	scope_time!("get_submodules");

	let (r, repo2) = (repo(repo_path)?, repo(repo_path)?);

	let res = r
		.submodules()?
		.iter()
		.map(|s| {
			let status = repo2
				.submodule_status(
					s.name().unwrap_or_default(),
					git2::SubmoduleIgnore::None,
				)
				.unwrap_or(SubmoduleStatus::empty());

			SubmoduleInfo {
				path: s.path().to_path_buf(),
				id: s.workdir_id().map(CommitId::from),
				head_id: s.head_id().map(CommitId::from),
				url: s.url().map(String::from),
				status,
			}
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
