//! asyncgit

#![forbid(missing_docs)]
#![deny(
	unused_imports,
	unused_must_use,
	dead_code,
	unstable_name_collisions,
	unused_assignments
)]
#![deny(unstable_name_collisions)]
#![deny(clippy::all, clippy::perf, clippy::nursery, clippy::pedantic)]
#![deny(clippy::filetype_is_file)]
#![deny(clippy::cargo)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::match_like_matches_macro)]
#![deny(clippy::needless_update)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
//TODO: get this in someday since expect still leads us to crashes sometimes
// #![deny(clippy::expect_used)]
//TODO: consider cleaning some up and allow specific places
#![allow(clippy::significant_drop_tightening)]

pub mod asyncjob;
mod blame;
mod branches;
pub mod cached;
mod commit_files;
mod diff;
mod error;
mod fetch_job;
mod progress;
mod pull;
mod push;
mod push_tags;
pub mod remote_progress;
pub mod remote_tags;
mod revlog;
mod status;
pub mod sync;
mod tags;
mod treefiles;

pub use crate::{
	blame::{AsyncBlame, BlameParams},
	branches::AsyncBranchesJob,
	commit_files::{AsyncCommitFiles, CommitFilesParams},
	diff::{AsyncDiff, DiffParams, DiffType},
	error::{Error, Result},
	fetch_job::AsyncFetchJob,
	progress::ProgressPercent,
	pull::{AsyncPull, FetchRequest},
	push::{AsyncPush, PushRequest},
	push_tags::{AsyncPushTags, PushTagsRequest},
	remote_progress::{RemoteProgress, RemoteProgressState},
	revlog::{AsyncLog, FetchStatus},
	status::{AsyncStatus, StatusParams},
	sync::{
		diff::{DiffLine, DiffLineType, FileDiff},
		remotes::push::PushType,
		status::{StatusItem, StatusItemType},
	},
	tags::AsyncTags,
	treefiles::AsyncTreeFilesJob,
};
pub use git2::message_prettify;
use std::{
	collections::hash_map::DefaultHasher,
	hash::{Hash, Hasher},
};

/// this type is used to communicate events back through the channel
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AsyncGitNotification {
	/// this indicates that no new state was fetched but that a async process finished
	FinishUnchanged,
	///
	Status,
	///
	Diff,
	///
	Log,
	///
	FileLog,
	///
	CommitFiles,
	///
	Tags,
	///
	Push,
	///
	PushTags,
	///
	Pull,
	///
	Blame,
	///
	RemoteTags,
	///
	Fetch,
	///
	Branches,
	///
	TreeFiles,
}

/// helper function to calculate the hash of an arbitrary type that implements the `Hash` trait
pub fn hash<T: Hash + ?Sized>(v: &T) -> u64 {
	let mut hasher = DefaultHasher::new();
	v.hash(&mut hasher);
	hasher.finish()
}

///
#[cfg(feature = "trace-libgit")]
pub fn register_tracing_logging() -> bool {
	fn git_trace(level: git2::TraceLevel, msg: &str) {
		log::info!("[{:?}]: {}", level, msg);
	}
	git2::trace_set(git2::TraceLevel::Trace, git_trace)
}

///
#[cfg(not(feature = "trace-libgit"))]
pub fn register_tracing_logging() -> bool {
	true
}
