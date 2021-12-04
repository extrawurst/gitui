use crate::{
	error::Result,
	hash,
	sync::{self, RepoPath},
	AsyncGitNotification,
};
use crossbeam_channel::Sender;
use std::{
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc, Mutex,
	},
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
	last: Arc<Mutex<Option<(Instant, TagsResult)>>>,
	sender: Sender<AsyncGitNotification>,
	pending: Arc<AtomicUsize>,
	repo: RepoPath,
}

impl AsyncTags {
	///
	pub fn new(
		repo: RepoPath,
		sender: &Sender<AsyncGitNotification>,
	) -> Self {
		Self {
			repo,
			last: Arc::new(Mutex::new(None)),
			sender: sender.clone(),
			pending: Arc::new(AtomicUsize::new(0)),
		}
	}

	/// last fetched result
	pub fn last(&mut self) -> Result<Option<Tags>> {
		let last = self.last.lock()?;

		Ok(last.clone().map(|last| last.1.tags))
	}

	///
	pub fn is_pending(&self) -> bool {
		self.pending.load(Ordering::Relaxed) > 0
	}

	fn is_outdated(&self, dur: Duration) -> Result<bool> {
		let last = self.last.lock()?;

		Ok(last
			.as_ref()
			.map_or(true, |(last_time, _)| last_time.elapsed() > dur))
	}

	///
	pub fn request(
		&mut self,
		dur: Duration,
		force: bool,
	) -> Result<()> {
		log::trace!("request");

		if !force && self.is_pending() {
			return Ok(());
		}

		let outdated = self.is_outdated(dur)?;

		if !force && !outdated {
			return Ok(());
		}

		let arc_last = Arc::clone(&self.last);
		let sender = self.sender.clone();
		let arc_pending = Arc::clone(&self.pending);

		self.pending.fetch_add(1, Ordering::Relaxed);
		let repo = self.repo.clone();

		rayon_core::spawn(move || {
			let notify = Self::getter(&repo, &arc_last, outdated)
				.expect("error getting tags");

			arc_pending.fetch_sub(1, Ordering::Relaxed);

			sender
				.send(if notify {
					AsyncGitNotification::Tags
				} else {
					AsyncGitNotification::FinishUnchanged
				})
				.expect("error sending notify");
		});

		Ok(())
	}

	fn getter(
		repo: &RepoPath,
		arc_last: &Arc<Mutex<Option<(Instant, TagsResult)>>>,
		outdated: bool,
	) -> Result<bool> {
		let tags = sync::get_tags(repo)?;

		let hash = hash(&tags);

		if !outdated
			&& Self::last_hash(arc_last)
				.map(|last| last == hash)
				.unwrap_or_default()
		{
			return Ok(false);
		}

		{
			let mut last = arc_last.lock()?;
			let now = Instant::now();
			*last = Some((now, TagsResult { hash, tags }));
		}

		Ok(true)
	}

	fn last_hash(
		last: &Arc<Mutex<Option<(Instant, TagsResult)>>>,
	) -> Option<u64> {
		last.lock()
			.ok()
			.and_then(|last| last.as_ref().map(|(_, last)| last.hash))
	}
}
