use anyhow::{Error, Result};
use asyncgit::{
	sync::{self, CommitInfo, RepoPathRef, Tags},
	AsyncGitNotification, AsyncLog, AsyncTags,
};
use bitflags::bitflags;
use crossbeam_channel::Sender;
use std::{borrow::Cow, convert::TryFrom, marker::PhantomData};
use std::{
	sync::{
		atomic::{AtomicBool, AtomicUsize, Ordering},
		Arc, Mutex,
	},
	thread,
	time::Duration,
};
use unicode_truncate::UnicodeTruncateStr;

const FILTER_SLEEP_DURATION: Duration = Duration::from_millis(10);
const SLICE_SIZE: usize = 1200;

bitflags! {
	pub struct FilterBy: u32 {
		const SHA = 0b0000_0001;
		const AUTHOR = 0b0000_0010;
		const MESSAGE = 0b0000_0100;
		const NOT = 0b0000_1000;
		const CASE_SENSITIVE = 0b0001_0000;
		const TAGS = 0b0010_0000;
	}
}

impl FilterBy {
	pub fn everywhere() -> Self {
		Self::all() & !Self::NOT & !Self::CASE_SENSITIVE
	}

	pub fn exclude_modifiers(self) -> Self {
		self & !Self::NOT & !Self::CASE_SENSITIVE
	}
}

impl Default for FilterBy {
	fn default() -> Self {
		Self::all() & !Self::NOT & !Self::CASE_SENSITIVE
	}
}

impl TryFrom<char> for FilterBy {
	type Error = anyhow::Error;

	fn try_from(v: char) -> Result<Self, Self::Error> {
		match v {
			's' => Ok(Self::SHA),
			'a' => Ok(Self::AUTHOR),
			'm' => Ok(Self::MESSAGE),
			'!' => Ok(Self::NOT),
			'c' => Ok(Self::CASE_SENSITIVE),
			't' => Ok(Self::TAGS),
			_ => Err(anyhow::anyhow!("Unknown flag: {v}")),
		}
	}
}

pub struct AsyncCommitFilterer {
	repo: RepoPathRef,
	git_log: AsyncLog,
	git_tags: AsyncTags,
	filtered_commits: Arc<Mutex<Vec<CommitInfo>>>,
	filter_count: Arc<AtomicUsize>,
	/// True if the filter thread is currently not running.
	filter_finished: Arc<AtomicBool>,
	/// Tells the last filter thread to stop early when set to true.
	filter_stop_signal: Arc<AtomicBool>,
	filter_thread_mutex: Arc<Mutex<()>>,
	sender: Sender<AsyncGitNotification>,

	/// `start_filter` logic relies on it being non-reentrant.
	_non_sync: PhantomData<std::cell::Cell<()>>,
}

impl AsyncCommitFilterer {
	pub fn new(
		repo: RepoPathRef,
		git_log: AsyncLog,
		git_tags: AsyncTags,
		sender: &Sender<AsyncGitNotification>,
	) -> Self {
		Self {
			repo,
			git_log,
			git_tags,
			filtered_commits: Arc::new(Mutex::new(Vec::new())),
			filter_count: Arc::new(AtomicUsize::new(0)),
			filter_finished: Arc::new(AtomicBool::new(true)),
			filter_thread_mutex: Arc::new(Mutex::new(())),
			filter_stop_signal: Arc::new(AtomicBool::new(false)),
			sender: sender.clone(),
			_non_sync: PhantomData,
		}
	}

	pub fn is_pending(&self) -> bool {
		!self.filter_finished.load(Ordering::Relaxed)
	}

	/// `filter_strings` should be split by or them and, for example,
	///
	/// A || B && C && D || E
	///
	/// would be
	///
	/// vec [vec![A], vec![B, C, D], vec![E]]
	#[allow(clippy::too_many_lines)]
	pub fn filter(
		vec_commit_info: Vec<CommitInfo>,
		tags: &Option<Tags>,
		filter_strings: &[Vec<(String, FilterBy)>],
	) -> Vec<CommitInfo> {
		vec_commit_info
			.into_iter()
			.filter(|commit| {
				Self::filter_one(filter_strings, tags, commit)
			})
			.collect()
	}

	fn filter_one(
		filter_strings: &[Vec<(String, FilterBy)>],
		tags: &Option<Tags>,
		commit: &CommitInfo,
	) -> bool {
		for to_and in filter_strings {
			if Self::filter_and(to_and, tags, commit) {
				return true;
			}
		}
		false
	}

	fn filter_and(
		to_and: &Vec<(String, FilterBy)>,
		tags: &Option<Tags>,
		commit: &CommitInfo,
	) -> bool {
		for (s, filter) in to_and {
			let by_sha = filter.contains(FilterBy::SHA);
			let by_aut = filter.contains(FilterBy::AUTHOR);
			let by_mes = filter.contains(FilterBy::MESSAGE);
			let by_tag = filter.contains(FilterBy::TAGS);

			let id: String;
			let author: Cow<str>;
			let message: Cow<str>;
			if filter.contains(FilterBy::CASE_SENSITIVE) {
				id = commit.id.to_string();
				author = Cow::Borrowed(&commit.author);
				message = Cow::Borrowed(&commit.message);
			} else {
				id = commit.id.to_string().to_lowercase();
				author = Cow::Owned(commit.author.to_lowercase());
				message = Cow::Owned(commit.message.to_lowercase());
			};

			let is_match = {
				let tag_contains = tags.as_ref().map_or(false, |t| {
					t.get(&commit.id).map_or(false, |commit_tags| {
						commit_tags
							.iter()
							.filter(|tag| tag.name.contains(s))
							.count() > 0
					})
				});

				(by_tag && tag_contains)
					|| (by_sha && id.contains(s))
					|| (by_aut && author.contains(s))
					|| (by_mes && message.contains(s))
			};

			let is_match = if filter.contains(FilterBy::NOT) {
				!is_match
			} else {
				is_match
			};

			if !is_match {
				return false;
			}
		}
		true
	}

	/// If the filtering string contain filtering by tags
	/// return them, else don't get the tags
	fn get_tags(
		filter_strings: &[Vec<(String, FilterBy)>],
		git_tags: &mut AsyncTags,
	) -> Result<Option<Tags>> {
		let mut contains_tags = false;
		for or in filter_strings {
			for (_, filter_by) in or {
				if filter_by.contains(FilterBy::TAGS) {
					contains_tags = true;
					break;
				}
			}
			if contains_tags {
				break;
			}
		}

		if contains_tags {
			return git_tags.last().map_err(|e| anyhow::anyhow!(e));
		}
		Ok(None)
	}

	pub fn start_filter(
		&mut self,
		filter_strings: Vec<Vec<(String, FilterBy)>>,
	) -> Result<()> {
		self.stop_filter();

		// `stop_filter` blocks until the previous threads finish, and
		// Self is !Sync, so two threads cannot be spawn at the same
		// time.
		//
		// We rely on these assumptions to keep `filtered_commits`
		// consistent.

		let filtered_commits = Arc::clone(&self.filtered_commits);

		filtered_commits.lock().expect("mutex poisoned").clear();

		let filter_count = Arc::clone(&self.filter_count);
		let async_log = self.git_log.clone();
		let filter_finished = Arc::clone(&self.filter_finished);

		self.filter_stop_signal = Arc::new(AtomicBool::new(false));
		let filter_stop_signal = Arc::clone(&self.filter_stop_signal);

		let async_app_sender = self.sender.clone();

		let filter_thread_mutex =
			Arc::clone(&self.filter_thread_mutex);

		let tags =
			Self::get_tags(&filter_strings, &mut self.git_tags)?;

		let repo = self.repo.clone();

		#[allow(clippy::significant_drop_tightening)]
		rayon_core::spawn(move || {
			// Only 1 thread can filter at a time
			let _c =
				filter_thread_mutex.lock().expect("mutex poisoned");

			filter_finished.store(false, Ordering::Relaxed);
			filter_count.store(0, Ordering::Relaxed);
			let mut cur_index: usize = 0;
			let result = loop {
				if filter_stop_signal.load(Ordering::Relaxed) {
					break Ok(());
				}

				// Get the git_log and start filtering through it
				let ids = match async_log
					.get_slice(cur_index, SLICE_SIZE)
				{
					Ok(ids) => ids,
					// Only errors if the lock is poisoned
					Err(err) => break Err(err),
				};

				let v = match sync::get_commits_info(
					&repo.borrow(),
					&ids,
					usize::MAX,
				) {
					Ok(v) => v,
					// May error while querying the repo or commits
					Err(err) => break Err(err),
				};

				// Assume finished if log not pending and 0 recieved
				if v.is_empty() && !async_log.is_pending() {
					break Ok(());
				}

				let mut filtered =
					Self::filter(v, &tags, &filter_strings);
				filter_count
					.fetch_add(filtered.len(), Ordering::Relaxed);

				filtered_commits
					.lock()
					.expect("mutex poisoned")
					.append(&mut filtered);

				cur_index += SLICE_SIZE;
				async_app_sender
					.send(AsyncGitNotification::Log)
					.expect("error sending");

				thread::sleep(FILTER_SLEEP_DURATION);
			};

			filter_finished.store(true, Ordering::Relaxed);

			if let Err(e) = result {
				log::error!("async job error: {}", e);
			}
		});
		Ok(())
	}

	/// Stop the filter thread if one was running, otherwise does nothing. This blocks until the
	/// filter thread is finished.
	pub fn stop_filter(&self) {
		self.filter_stop_signal.store(true, Ordering::Relaxed);

		// wait for the filter thread to finish
		drop(self.filter_thread_mutex.lock());
	}

	pub fn get_filter_items(
		&mut self,
		start: usize,
		amount: usize,
		message_length_limit: usize,
	) -> Result<Vec<CommitInfo>> {
		let mut commits_requested = {
			let fc = self
				.filtered_commits
				.lock()
				.map_err(|_| Error::msg("mutex poisoned"))?;
			let len = fc.len();
			let min = start.min(len);
			let max = min + amount;
			let max = max.min(len);

			fc[min..max].to_vec()
		};

		for c in &mut commits_requested {
			c.message = c
				.message
				.unicode_truncate(message_length_limit)
				.0
				.to_owned();
		}
		Ok(commits_requested)
	}

	pub fn count(&self) -> usize {
		self.filter_count.load(Ordering::Relaxed)
	}
}

#[cfg(test)]
mod test {
	use asyncgit::sync::{CommitId, CommitInfo};

	use crate::tabs::Revlog;

	use super::AsyncCommitFilterer;

	fn commit(
		time: i64,
		message: &str,
		author: &str,
		id: &str,
	) -> CommitInfo {
		CommitInfo {
			message: message.to_string(),
			time,
			author: author.to_string(),
			id: CommitId::from_hex_str(id)
				.expect("invalid commit id"),
		}
	}

	fn filter(
		commits: Vec<CommitInfo>,
		filter: &str,
	) -> Vec<CommitInfo> {
		let filter_string = Revlog::get_what_to_filter_by(filter);
		dbg!(&filter_string);
		AsyncCommitFilterer::filter(commits, &None, &filter_string)
	}

	#[test]
	fn test_filter() {
		let commits = vec![
			commit(0, "a", "b", "0"),
			commit(1, "0", "0", "a"),
			commit(2, "0", "A", "b"),
			commit(3, "0", "0", "0"),
		];

		let filtered = |indices: &[usize]| {
			indices
				.iter()
				.map(|i| commits[*i].clone())
				.collect::<Vec<_>>()
		};

		assert_eq!(
			filter(commits.clone(), "a"), //
			filtered(&[0, 1, 2])
		);
		assert_eq!(
			filter(commits.clone(), "A"), //
			filtered(&[0, 1, 2])
		);

		assert_eq!(
			filter(commits.clone(), ":m a"), //
			filtered(&[0]),
		);
		assert_eq!(
			filter(commits.clone(), ":s a"), //
			filtered(&[1]),
		);
		assert_eq!(
			filter(commits.clone(), ":a a"), //
			filtered(&[2]),
		);

		assert_eq!(
			filter(commits.clone(), ":! a"), //
			filtered(&[3]),
		);

		assert_eq!(
			filter(commits.clone(), ":!m a"), //
			filtered(&[1, 2, 3]),
		);
		assert_eq!(
			filter(commits.clone(), ":!s a"), //
			filtered(&[0, 2, 3]),
		);
		assert_eq!(
			filter(commits.clone(), ":!a a"), //
			filtered(&[0, 1, 3]),
		);

		assert_eq!(
			filter(commits.clone(), "a && b"), //
			filtered(&[0, 2]),
		);

		assert_eq!(
			filter(commits.clone(), ":m a && :a b"), //
			filtered(&[0]),
		);
		assert_eq!(
			filter(commits.clone(), "b && :!m a"), //
			filtered(&[2]),
		);
		assert_eq!(
			filter(commits.clone(), ":! b && a"), //
			filtered(&[1]),
		);
		assert_eq!(
			filter(commits.clone(), ":! b && :! a"), //
			filtered(&[3]),
		);

		assert_eq!(
			filter(commits.clone(), ":c a"), //
			filtered(&[0, 1]),
		);
		assert_eq!(
			filter(commits.clone(), ":c A"), //
			filtered(&[2]),
		);
		assert_eq!(
			filter(commits.clone(), ":!c a"), //
			filtered(&[2, 3]),
		);
	}
}
