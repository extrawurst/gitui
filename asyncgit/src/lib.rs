//! asyncgit

#![forbid(unsafe_code)]
#![forbid(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::perf)]

pub mod cached;
mod commit_files;
mod diff;
mod error;
mod push;
mod revlog;
mod status;
pub mod sync;
mod tags;

pub use crate::{
    commit_files::AsyncCommitFiles,
    diff::{AsyncDiff, DiffParams, DiffType},
    push::{AsyncPush, PushProgress, PushProgressState, PushRequest},
    revlog::{AsyncLog, FetchStatus},
    status::{AsyncStatus, StatusParams},
    sync::{
        diff::{DiffLine, DiffLineType, FileDiff},
        status::{StatusItem, StatusItemType},
    },
    tags::AsyncTags,
};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

/// this type is used to communicate events back through the channel
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AsyncNotification {
    /// this indicates that no new state was fetched but that a async process finished
    FinishUnchanged,
    ///
    Status,
    ///
    Diff,
    ///
    Log,
    ///
    CommitFiles,
    ///
    Tags,
    ///
    Push,
}

/// current working director `./`
pub static CWD: &str = "./";

/// helper function to calculate the hash of an arbitrary type that implements the `Hash` trait
pub fn hash<T: Hash + ?Sized>(v: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    v.hash(&mut hasher);
    hasher.finish()
}
