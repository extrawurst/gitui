use asyncgit::sync::{CommitId, CommitInfo};
use chrono::prelude::*;
use std::slice::Iter;

static SLICE_OFFSET_RELOAD_THRESHOLD: usize = 100;

pub struct LogEntry {
    pub time: String,
    pub author: String,
    pub msg: String,
    pub hash_short: String,
    pub id: CommitId,
}

impl From<CommitInfo> for LogEntry {
    fn from(c: CommitInfo) -> Self {
        let time =
            DateTime::<Local>::from(DateTime::<Utc>::from_utc(
                NaiveDateTime::from_timestamp(c.time, 0),
                Utc,
            ));

        let hash = c.id.to_string().chars().take(7).collect();

        Self {
            author: c.author,
            msg: c.message,
            time: time.format("%Y-%m-%d %H:%M:%S").to_string(),
            hash_short: hash,
            id: c.id,
        }
    }
}

///
#[derive(Default)]
pub struct ItemBatch {
    index_offset: usize,
    items: Vec<LogEntry>,
}

impl ItemBatch {
    fn last_idx(&self) -> usize {
        self.index_offset + self.items.len()
    }

    ///
    pub const fn index_offset(&self) -> usize {
        self.index_offset
    }

    /// shortcut to get an `Iter` of our internal items
    pub fn iter(&self) -> Iter<'_, LogEntry> {
        self.items.iter()
    }

    /// clear curent list of items
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// insert new batch of items
    pub fn set_items(
        &mut self,
        start_index: usize,
        commits: Vec<CommitInfo>,
    ) {
        self.items.clear();
        self.items.extend(commits.into_iter().map(LogEntry::from));
        self.index_offset = start_index;
    }

    /// returns `true` if we should fetch updated list of items
    pub fn needs_data(&self, idx: usize, idx_max: usize) -> bool {
        let want_min =
            idx.saturating_sub(SLICE_OFFSET_RELOAD_THRESHOLD);
        let want_max = idx
            .saturating_add(SLICE_OFFSET_RELOAD_THRESHOLD)
            .min(idx_max);

        let needs_data_top = want_min < self.index_offset;
        let needs_data_bottom = want_max >= self.last_idx();
        needs_data_bottom || needs_data_top
    }
}
