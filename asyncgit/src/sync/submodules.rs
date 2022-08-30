//TODO:
// #![allow(unused_variables, dead_code)]

use std::path::PathBuf;

use git2::{Repository, Submodule, SubmoduleUpdateOptions};
use scopetime::scope_time;

use super::{repo, CommitId, RepoPath};
use crate::{error::Result, sync::utils::work_dir, Error};

pub use git2::SubmoduleStatus;

///
#[derive(Debug)]
pub struct SubmoduleInfo {
	///
	pub name: String,
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

///
#[derive(Debug)]
pub struct SubmoduleParentInfo {
	/// where to find parent repo
	pub parent_gitpath: PathBuf,
	/// where to find submodule git path
	pub submodule_gitpath: PathBuf,
	/// `submodule_info` from perspective of parent repo
	pub submodule_info: SubmoduleInfo,
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

fn submodule_to_info(s: &Submodule, r: &Repository) -> SubmoduleInfo {
	let status = r
		.submodule_status(
			s.name().unwrap_or_default(),
			git2::SubmoduleIgnore::None,
		)
		.unwrap_or(SubmoduleStatus::empty());

	SubmoduleInfo {
		name: s.name().unwrap_or_default().into(),
		path: s.path().to_path_buf(),
		id: s.workdir_id().map(CommitId::from),
		head_id: s.head_id().map(CommitId::from),
		url: s.url().map(String::from),
		status,
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
		.map(|s| submodule_to_info(s, &repo2))
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

/// query whether `repo_path` points to a repo that is part of a parent git which contains it as a submodule
pub fn submodule_parent_info(
	repo_path: &RepoPath,
) -> Result<Option<SubmoduleParentInfo>> {
	scope_time!("submodule_parent_info");

	let repo = repo(repo_path)?;
	let repo_wd = work_dir(&repo)?.to_path_buf();

	if let Some(parent_path) =
		repo.path().parent().and_then(std::path::Path::parent)
	{
		if let Ok(parent) = Repository::open(parent_path) {
			let parent_wd = work_dir(&parent)?.to_path_buf();

			let submodule_name = repo_wd
				.strip_prefix(parent_wd)?
				.to_string_lossy()
				.to_string();

			if let Ok(submodule) =
				parent.find_submodule(&submodule_name)
			{
				return Ok(Some(SubmoduleParentInfo {
					parent_gitpath: parent.path().to_path_buf(),
					submodule_gitpath: repo.path().to_path_buf(),
					submodule_info: submodule_to_info(
						&submodule, &parent,
					),
				}));
			}
		}
	}

	Ok(None)
}

#[cfg(test)]
mod tests {
	use git2::Repository;

	use crate::sync::{
		submodules::submodule_parent_info, tests::repo_init, RepoPath,
	};
	use std::path::Path;

	use super::get_submodules;

	#[test]
	fn test_smoke() {
		let (dir, _r) = repo_init().unwrap();

		{
			let r = Repository::open(dir.path()).unwrap();
			let mut s = r
				.submodule(
					//TODO: use local git
					"https://github.com/extrawurst/brewdump.git",
					Path::new("foo"),
					false,
				)
				.unwrap();

			let _sub_r = s.clone(None).unwrap();
			s.add_finalize().unwrap();
		}

		let repo_p = RepoPath::Path(dir.into_path());
		let subs = get_submodules(&repo_p).unwrap();

		assert_eq!(subs.len(), 1);
		assert_eq!(&subs[0].name, "foo");

		let info = submodule_parent_info(
			&subs[0].get_repo_path(&repo_p).unwrap(),
		)
		.unwrap()
		.unwrap();

		dbg!(&info);

		assert_eq!(&info.submodule_info.name, "foo");
	}
}
