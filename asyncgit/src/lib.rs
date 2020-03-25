//! asyncgit

#![forbid(unsafe_code)]
#![warn(missing_docs)]
mod diff;
mod status;
pub mod sync;

pub use crate::{
    diff::{AsyncDiff, DiffParams},
    status::AsyncStatus,
    sync::{
        diff::{Diff, DiffLine, DiffLineType},
        status::{StatusItem, StatusItemType},
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
}

/// helper function to calculate the hash of an arbitrary type that implements the `Hash` trait
pub fn hash<T: Hash>(v: &T) -> u64 {
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
