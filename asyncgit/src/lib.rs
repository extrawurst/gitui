//! asyncgit

#![forbid(missing_docs)]
#![deny(unsafe_code)]
#![deny(unused_imports)]
#![deny(unused_must_use)]
#![deny(dead_code)]
#![deny(clippy::all)]
#![deny(clippy::cargo)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::perf)]
#![deny(clippy::match_like_matches_macro)]
#![deny(clippy::needless_update)]
#![allow(clippy::module_name_repetitions)]
//TODO: get this in someday since expect still leads us to crashes sometimes
// #![deny(clippy::expect_used)]
//TODO:
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::redundant_pub_crate)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]

pub mod cached;
mod commit_files;
mod diff;
mod error;
mod fetch;
mod progress;
mod push;
mod push_tags;
pub mod remote_progress;
mod revlog;
mod status;
pub mod sync;
mod tags;

pub use crate::{
    commit_files::AsyncCommitFiles,
    diff::{AsyncDiff, DiffParams, DiffType},
    fetch::{AsyncFetch, FetchRequest},
    push::{AsyncPush, PushRequest},
    push_tags::{AsyncPushTags, PushTagsRequest},
    remote_progress::{RemoteProgress, RemoteProgressState},
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
    ///
    PushTags,
    ///
    Fetch,
}

/// current working director `./`
pub static CWD: &str = "./";

/// helper function to calculate the hash of an arbitrary type that implements the `Hash` trait
pub fn hash<T: Hash + ?Sized>(v: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    v.hash(&mut hasher);
    hasher.finish()
}
