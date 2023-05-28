use crate::{
	error::Result,
	sync::{self, CommitId, RepoPath},
	AsyncGitNotification,
};
use crossbeam_channel::Sender;
use std::{
	sync::atomic::{AtomicUsize, Ordering},
	sync::Arc,
};

///
pub struct AsyncStash {
	pending: Arc<AtomicUsize>,
	sender: Sender<AsyncGitNotification>,
	repo: RepoPath,
}

impl AsyncStash {
	///
	pub fn new(
		repo: RepoPath,
		sender: Sender<AsyncGitNotification>,
	) -> Self {
		Self {
			repo,
			sender,
			pending: Arc::new(AtomicUsize::new(0)),
		}
	}

	///
	pub fn stash_save(
		&mut self,
		message: Option<&str>,
		include_untracked: bool,
		keep_index: bool,
	) -> Result<Option<CommitId>> {
		if self.is_pending() {
			log::trace!("request blocked, still pending");
			return Ok(None);
		}

		let repo = self.repo.clone();
		let sender = self.sender.clone();
		let pending = self.pending.clone();
		let message = message.map(ToOwned::to_owned);

		self.pending.fetch_add(1, Ordering::Relaxed);

		rayon_core::spawn(move || {
			let res = sync::stash::stash_save(
				&repo,
				message.as_deref(),
				include_untracked,
				keep_index,
			);

			pending.fetch_sub(1, Ordering::Relaxed);

			sender
				.send(AsyncGitNotification::Stash)
				.expect("error sending stash notification");

			if let Err(e) = res {
				log::error!("AsyncStash error: {}", e);
			}
		});

		Ok(None)
	}

	///
	pub fn is_pending(&self) -> bool {
		self.pending.load(Ordering::Relaxed) > 0
	}
}
