//!

//TODO:
#![allow(dead_code)]

use crate::{
	asyncjob::{AsyncJob, RunParams},
	error::Result,
	sync::cred::BasicAuthCredential,
	sync::remotes::fetch_all,
	AsyncGitNotification, ProgressPercent, CWD,
};

use std::sync::{Arc, Mutex};

enum JobState {
	Request(Option<BasicAuthCredential>),
	Response(Result<()>),
}

///
#[derive(Clone, Default)]
pub struct AsyncFetchJob {
	state: Arc<Mutex<Option<JobState>>>,
}

///
impl AsyncFetchJob {
	///
	pub fn new(
		basic_credential: Option<BasicAuthCredential>,
	) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(JobState::Request(
				basic_credential,
			)))),
		}
	}

	///
	pub fn result(&self) -> Option<Result<()>> {
		if let Ok(mut state) = self.state.lock() {
			if let Some(state) = state.take() {
				return match state {
					JobState::Request(_) => None,
					JobState::Response(result) => Some(result),
				};
			}
		}

		None
	}
}

impl AsyncJob for AsyncFetchJob {
	type Notification = AsyncGitNotification;
	type Progress = ProgressPercent;

	fn run(
		&mut self,
		_params: RunParams<Self::Notification, Self::Progress>,
	) -> Result<Self::Notification> {
		if let Ok(mut state) = self.state.lock() {
			*state = state.take().map(|state| match state {
				JobState::Request(basic_credentials) => {
					let result =
						fetch_all(CWD, &basic_credentials, &None);

					JobState::Response(result)
				}
				JobState::Response(result) => {
					JobState::Response(result)
				}
			});
		}

		Ok(AsyncGitNotification::Fetch)
	}
}
