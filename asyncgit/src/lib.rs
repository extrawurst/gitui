//! asyncgit

#![forbid(unsafe_code)]
#![forbid(missing_docs)]
#![deny(clippy::all)]

mod diff;
mod error;
mod revlog;
mod status;
mod status2;
pub mod sync;

pub use crate::{
    diff::{AsyncDiff, DiffParams},
    revlog::AsyncLog,
    status::AsyncStatus,
    status2::{AsyncStatus2, StatusParams},
    sync::{
        diff::{DiffLine, DiffLineType, FileDiff},
        status::{StatusItem, StatusItemType},
        utils::is_repo,
    },
};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    time::{SystemTime, UNIX_EPOCH},
};

/// this type is used to communicate events back through the channel
#[derive(Copy, Clone, Debug)]
pub enum AsyncNotification {
    ///
    Status,
    ///
    Diff,
    ///
    Log,
}

/// current working director `./`
pub static CWD: &str = "./";

/// helper function to calculate the hash of an arbitrary type that implements the `Hash` trait
pub fn hash<T: Hash + ?Sized>(v: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    v.hash(&mut hasher);
    hasher.finish()
}

/// helper function to return the current tick since unix epoch
pub fn current_tick() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
