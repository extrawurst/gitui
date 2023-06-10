use asyncgit::sync::{CommitId, CommitInfo};
use chrono::{DateTime, Duration, Local, NaiveDateTime, Utc};
use std::slice::Iter;

#[cfg(feature = "ghemoji")]
use super::emoji::emojifi_string;

static SLICE_OFFSET_RELOAD_THRESHOLD: usize = 100;

type BoxStr = Box<str>;

pub struct LogEntry {
	//TODO: cache string representation
	pub time: DateTime<Local>,
	//TODO: use tinyvec here
	pub author: BoxStr,
	pub msg: BoxStr,
	//TODO: use tinyvec here
	pub hash_short: BoxStr,
	pub id: CommitId,
}

impl From<CommitInfo> for LogEntry {
	fn from(c: CommitInfo) -> Self {
		let hash_short = c.id.get_short_string().into();

		let time = {
			let date = NaiveDateTime::from_timestamp_opt(c.time, 0);
			if date.is_none() {
				log::error!("error reading commit date: {hash_short} - timestamp: {}",c.time);
			}
			DateTime::<Local>::from(DateTime::<Utc>::from_utc(
				date.unwrap_or_default(),
				Utc,
			))
		};

		let author = c.author;
		#[allow(unused_mut)]
		let mut msg = c.message;

		// Replace markdown emojis with Unicode equivalent
		#[cfg(feature = "ghemoji")]
		emojifi_string(&mut msg);

		Self {
			author: author.into(),
			msg: msg.into(),
			time,
			hash_short,
			id: c.id,
		}
	}
}

impl LogEntry {
	pub fn time_to_string(&self, now: DateTime<Local>) -> String {
		let delta = now - self.time;
		if delta < Duration::minutes(30) {
			let delta_str = if delta < Duration::minutes(1) {
				"<1m ago".to_string()
			} else {
				format!("{:0>2}m ago", delta.num_minutes())
			};
			format!("{delta_str: <10}")
		} else if self.time.date_naive() == now.date_naive() {
			self.time.format("%T  ").to_string()
		} else {
			self.time.format("%Y-%m-%d").to_string()
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

#[cfg(test)]
#[cfg(feature = "ghemoji")]
mod tests {
	use super::*;

	fn test_conversion(s: &str) -> String {
		let mut s = s.to_string();
		emojifi_string(&mut s);
		s
	}

	#[test]
	fn test_emojifi_string_conversion_cases() {
		assert_eq!(
			&test_conversion("It's :hammer: time!"),
			"It's 🔨 time!"
		);
		assert_eq!(
			&test_conversion(":red_circle::orange_circle::yellow_circle::green_circle::large_blue_circle::purple_circle:"),
			"🔴🟠🟡🟢🔵🟣"
		);
		assert_eq!(
			&test_conversion("It's raining :cat:s and :dog:s"),
			"It's raining 🐱s and 🐶s"
		);
		assert_eq!(&test_conversion(":crab: rules!"), "🦀 rules!");
	}

	#[test]
	fn test_emojifi_string_no_conversion_cases() {
		assert_eq!(&test_conversion("123"), "123");
		assert_eq!(
			&test_conversion("This :should_not_convert:"),
			"This :should_not_convert:"
		);
		assert_eq!(&test_conversion(":gopher:"), ":gopher:");
	}
}
