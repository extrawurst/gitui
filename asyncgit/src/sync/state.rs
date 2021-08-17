use crate::{error::Result, sync::utils};
use git2::RepositoryState;
use scopetime::scope_time;

///
#[derive(Debug, PartialEq)]
pub enum RepoState {
	///
	Clean,
	///
	Merge,
	///
	Other,
}

impl From<RepositoryState> for RepoState {
	fn from(state: RepositoryState) -> Self {
		match state {
			RepositoryState::Clean => Self::Clean,
			RepositoryState::Merge => Self::Merge,
			_ => Self::Other,
		}
	}
}

///
pub fn repo_state(repo_path: &str) -> Result<RepoState> {
	scope_time!("repo_state");

	let repo = utils::repo(repo_path)?;

	Ok(repo.state().into())
}
