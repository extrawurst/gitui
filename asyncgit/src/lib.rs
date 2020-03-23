mod diff;
mod status;
pub mod sync;

pub use crate::{
    diff::AsyncDiff,
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

#[derive(Copy, Clone, Debug)]
pub enum AsyncNotification {
    Status,
    Diff,
}

pub fn hash<T: Hash>(v: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    v.hash(&mut hasher);
    hasher.finish()
}

pub fn current_tick() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
