use crate::{
	asyncjob::{AsyncJob, RunParams},
	error::Result,
	sync::{self, CommitId, LogWalkerFilter, RepoPath},
	AsyncGitNotification, ProgressPercent,
};
use std::{
	sync::{Arc, Mutex},
	time::{Duration, Instant},
};

///
pub struct CommitFilterResult {
	///
	pub result: Vec<CommitId>,
	///
	pub duration: Duration,
}

enum JobState {
	Request {
		commits: Vec<CommitId>,
		repo_path: RepoPath,
	},
	Response(Result<CommitFilterResult>),
}

///
#[derive(Clone)]
pub struct AsyncCommitFilterJob {
	state: Arc<Mutex<Option<JobState>>>,
	filter: LogWalkerFilter,
}

///
impl AsyncCommitFilterJob {
	///
	pub fn new(
		repo_path: RepoPath,
		commits: Vec<CommitId>,
		filter: LogWalkerFilter,
	) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(JobState::Request {
				repo_path,
				commits,
			}))),
			filter,
		}
	}

	///
	pub fn result(&self) -> Option<Result<CommitFilterResult>> {
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

	fn run_request(
		&self,
		repo_path: &RepoPath,
		commits: Vec<CommitId>,
		params: &RunParams<AsyncGitNotification, ProgressPercent>,
	) -> JobState {
		let response = sync::repo(repo_path)
			.map(|repo| self.filter_commits(&repo, commits, params))
			.map(|(start, result)| CommitFilterResult {
				result,
				duration: start.elapsed(),
			});

		JobState::Response(response)
	}

	fn filter_commits(
		&self,
		repo: &git2::Repository,
		commits: Vec<CommitId>,
		params: &RunParams<AsyncGitNotification, ProgressPercent>,
	) -> (Instant, Vec<CommitId>) {
		let total_amount = commits.len();
		let start = Instant::now();

		let mut progress = ProgressPercent::new(0, total_amount);

		let result = commits
			.into_iter()
			.enumerate()
			.filter_map(|(idx, c)| {
				let new_progress =
					ProgressPercent::new(idx, total_amount);

				if new_progress != progress {
					Self::update_progress(params, new_progress);
					progress = new_progress;
				}

				(*self.filter)(repo, &c)
					.ok()
					.and_then(|res| res.then_some(c))
			})
			.collect::<Vec<_>>();

		(start, result)
	}

	fn update_progress(
		params: &RunParams<AsyncGitNotification, ProgressPercent>,
		new_progress: ProgressPercent,
	) {
		if let Err(e) = params.set_progress(new_progress) {
			log::error!("progress error: {e}");
		} else if let Err(e) =
			params.send(AsyncGitNotification::CommitFilter)
		{
			log::error!("send error: {e}");
		}
	}
}

impl AsyncJob for AsyncCommitFilterJob {
	type Notification = AsyncGitNotification;
	type Progress = ProgressPercent;

	fn run(
		&mut self,
		params: RunParams<Self::Notification, Self::Progress>,
	) -> Result<Self::Notification> {
		if let Ok(mut state) = self.state.lock() {
			*state = state.take().map(|state| match state {
				JobState::Request { commits, repo_path } => {
					self.run_request(&repo_path, commits, &params)
				}
				JobState::Response(result) => {
					JobState::Response(result)
				}
			});
		}

		Ok(AsyncGitNotification::CommitFilter)
	}
}
