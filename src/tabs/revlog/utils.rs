use asyncgit::sync::CommitInfo;
use chrono::prelude::*;

static SLICE_OFFSET_RELOAD_THRESHOLD: usize = 100;

#[derive(Default)]
pub(super) struct LogEntry {
    pub time: String,
    pub author: String,
    pub msg: String,
    pub hash: String,
}

impl From<CommitInfo> for LogEntry {
    fn from(c: CommitInfo) -> Self {
        let time =
            DateTime::<Local>::from(DateTime::<Utc>::from_utc(
                NaiveDateTime::from_timestamp(c.time, 0),
                Utc,
            ));
        Self {
            author: c.author,
            msg: c.message,
            time: time.format("%Y-%m-%d %H:%M:%S").to_string(),
            hash: c.hash,
        }
    }
}

///
#[derive(Default)]
pub(super) struct ItemBatch {
    pub index_offset: usize,
    pub items: Vec<LogEntry>,
}

impl ItemBatch {
    fn last_idx(&self) -> usize {
        self.index_offset + self.items.len()
    }

    pub fn set_items(
        &mut self,
        start_index: usize,
        commits: Vec<CommitInfo>,
    ) {
        self.items.clear();
        self.items.extend(commits.into_iter().map(LogEntry::from));
        self.index_offset = start_index;
    }

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
