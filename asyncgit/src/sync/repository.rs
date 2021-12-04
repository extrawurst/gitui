use std::path::{Path, PathBuf};

use git2::{Repository, RepositoryOpenFlags};

use crate::error::Result;

///
#[derive(Clone)]
pub enum RepoPath {
	///
	Path(PathBuf),
	// Workdir { gitdir: PathBuf, workdir:PathBuf },
}

impl RepoPath {
	///
	pub fn gitpath(&self) -> &Path {
		match self {
			RepoPath::Path(p) => p.as_path(),
		}
	}
}

impl From<&str> for RepoPath {
	fn from(p: &str) -> Self {
		Self::Path(PathBuf::from(p))
	}
}

pub fn repo(repo_path: &RepoPath) -> Result<Repository> {
	let repo = Repository::open_ext(
		repo_path.gitpath(),
		RepositoryOpenFlags::empty(),
		Vec::<&Path>::new(),
	)?;

	if repo.is_bare() {
		// repo.set_workdir(&Path::new("/Users/stephan/code/"), false)?;
	}

	Ok(repo)
}
