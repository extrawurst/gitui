use crate::{
	asyncjob::{AsyncJob, AsyncSingleJob, RunParams},
	error::Result,
	sync::{self},
	AsyncGitNotification, CWD,
};
use crossbeam_channel::Sender;
use std::{
	sync::{Arc, Mutex},
	time::{Duration, Instant},
};
use sync::Tags;

///
#[derive(Default, Clone)]
struct TagsResult {
	hash: u64,
	tags: Tags,
}

///
pub struct AsyncTags {
	last: Option<(Instant, TagsResult)>,
	sender: Sender<AsyncGitNotification>,
	job: AsyncSingleJob<AsyncTagsJob>,
}

impl AsyncTags {
	///
	pub fn new(sender: &Sender<AsyncGitNotification>) -> Self {
		Self {
			last: None,
			sender: sender.clone(),
			job: AsyncSingleJob::new(sender.clone()),
		}
	}

	/// last fetched result
	pub fn last(&self) -> Result<Option<Tags>> {
		Ok(self.last.as_ref().map(|result| result.1.tags.clone()))
	}

	///
	pub fn is_pending(&self) -> bool {
		self.job.is_pending()
	}

	///
	fn is_outdated(&self, dur: Duration) -> bool {
		self.last
			.as_ref()
			.map_or(true, |(last_time, _)| last_time.elapsed() > dur)
	}

	///
	pub fn request(
		&mut self,
		dur: Duration,
		force: bool,
	) -> Result<()> {
		log::trace!("request");

		if !force && self.job.is_pending() {
			return Ok(());
		}

		let outdated = self.is_outdated(dur);

		if !force && !outdated {
			return Ok(());
		}

		if outdated {
			self.job.spawn(AsyncTagsJob::new());
		} else {
			self.sender
				.send(AsyncGitNotification::FinishUnchanged)?;
		}

		Ok(())
	}
}

enum JobState {
	Request(),
	Response(Result<(Instant, TagsResult)>),
}

///
#[derive(Clone, Default)]
pub struct AsyncTagsJob {
	state: Arc<Mutex<Option<JobState>>>,
}

///
impl AsyncTagsJob {
	///
	pub fn new() -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(JobState::Request()))),
		}
	}
}

impl AsyncJob for AsyncTagsJob {
	type Notification = AsyncGitNotification;
	type Progress = ();

	fn run(
		&mut self,
		_params: RunParams<Self::Notification, Self::Progress>,
	) -> Result<Self::Notification> {
		if let Ok(mut state) = self.state.lock() {
			*state = state.take().map(|state| match state {
				JobState::Request() => {
					let tags = sync::get_tags(CWD);
					// let hash = tags.hash();
					let hash = Ok(0);

					JobState::Response(tags.and_then(|tags| {
						hash.map(|hash| {
							(
								Instant::now(),
								TagsResult { hash, tags },
							)
						})
					}))
				}
				JobState::Response(result) => {
					JobState::Response(result)
				}
			});
		}

		Ok(AsyncGitNotification::Tags)
	}
}
