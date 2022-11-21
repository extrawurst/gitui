use crate::{
	asyncjob::{AsyncJob, RunParams},
	error::Result,
	sync::{tree_files, CommitId, RepoPath, TreeFile},
	AsyncGitNotification,
};
use std::sync::{Arc, Mutex};

enum JobState {
	Request { commit: CommitId, repo: RepoPath },
	Response(Result<Vec<TreeFile>>),
}

///
#[derive(Clone, Default)]
pub struct AsyncTreeFilesJob {
	state: Arc<Mutex<Option<JobState>>>,
}

///
impl AsyncTreeFilesJob {
	///
	pub fn new(repo: RepoPath, commit: CommitId) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(JobState::Request {
				repo,
				commit,
			}))),
		}
	}

	///
	pub fn result(&self) -> Option<Result<Vec<TreeFile>>> {
		if let Ok(mut state) = self.state.lock() {
			if let Some(state) = state.take() {
				return match state {
					JobState::Request { .. } => None,
					JobState::Response(result) => Some(result),
				};
			}
		}

		None
	}
}

impl AsyncJob for AsyncTreeFilesJob {
	type Notification = AsyncGitNotification;
	type Progress = ();

	fn run(
		&mut self,
		_params: RunParams<Self::Notification, Self::Progress>,
	) -> Result<Self::Notification> {
		if let Ok(mut state) = self.state.lock() {
			*state = state.take().map(|state| match state {
				JobState::Request { commit, repo } => {
					let files = tree_files(&repo, commit);

					std::thread::sleep(
						std::time::Duration::from_secs(2),
					);
					JobState::Response(files)
				}
				JobState::Response(result) => {
					JobState::Response(result)
				}
			});
		}

		Ok(AsyncGitNotification::TreeFiles)
	}
}
