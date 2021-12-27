//!

use crate::asyncjob::{AsyncJob, RunParams};
use crate::error::Result;
use crate::sync::CommitId;
use crate::AsyncGitNotification;
use crate::StatusItemType;
use std::sync::Arc;
use std::sync::Mutex;

enum JobState {
	Request(String),
	Response(Result<Vec<(CommitId, StatusItemType)>>),
}

///
#[derive(Clone, Default)]
pub struct AsyncFileLogJob {
	state: Arc<Mutex<Option<JobState>>>,
}

impl AsyncFileLogJob {
	///
	pub fn new(file_path: &str) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(JobState::Request(
				file_path.into(),
			)))),
		}
	}
}

impl AsyncJob for AsyncFileLogJob {
	type Notification = AsyncGitNotification;
	type Progress = ();

	fn run(
		&mut self,
		_params: RunParams<Self::Notification, Self::Progress>,
	) -> Result<Self::Notification> {
		if let Ok(mut state) = self.state.lock() {
			*state = Some(JobState::Response(Ok(vec![])));
		}

		Ok(AsyncGitNotification::FileLog)
	}
}

// fn get_file_status(&self, commit_id: CommitId) -> char {
// 	self.file_path
// 		.as_ref()
// 		.and_then(|file_path| {
// 			let repo = repo(CWD);

// 			repo.ok().and_then(|repo| {
// 				let diff = get_commit_diff(
// 					&repo,
// 					commit_id,
// 					Some(file_path.clone()),
// 				);

// 				diff.ok().and_then(|diff| {
// 					diff.deltas().next().map(|delta| {
// 						let status: StatusItemType =
// 							delta.status().into();

// 						match status {
// 							StatusItemType::New => 'A',
// 							StatusItemType::Deleted => 'D',
// 							_ => 'M',
// 						}
// 					})
// 				})
// 			})
// 		})
// 		.unwrap_or(' ')
// }
