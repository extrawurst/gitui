use super::utils::logitems::{ItemBatch, LogEntry};
use crate::{
	components::{
		utils::string_width_align, CommandBlocking, CommandInfo,
		Component, DrawableComponent, EventState, ScrollType,
	},
	keys::{key_match, SharedKeyConfig},
	queue::{InternalEvent, Queue},
	strings::{self, symbol},
	try_or_popup,
	ui::style::{SharedTheme, Theme},
	ui::{calc_scroll_top, draw_scrollbar, Orientation},
};
use anyhow::Result;
use asyncgit::sync::{
	checkout_commit, BranchDetails, BranchInfo, CommitId,
	RepoPathRef, Tags,
};
use chrono::{DateTime, Local};
use crossterm::event::Event;
use itertools::Itertools;
use ratatui::{
	backend::Backend,
	layout::{Alignment, Rect},
	text::{Span, Spans},
	widgets::{Block, Borders, Paragraph},
	Frame,
};
use std::{
	borrow::Cow, cell::Cell, cmp, collections::BTreeMap,
	convert::TryFrom, time::Instant,
};

const ELEMENTS_PER_LINE: usize = 9;

///
pub struct CommitList {
	repo: RepoPathRef,
	title: Box<str>,
	selection: usize,
	count_total: usize,
	items: ItemBatch,
	marked: Vec<(usize, CommitId)>,
	scroll_state: (Instant, f32),
	tags: Option<Tags>,
	local_branches: BTreeMap<CommitId, Vec<BranchInfo>>,
	remote_branches: BTreeMap<CommitId, Vec<BranchInfo>>,
	current_size: Cell<Option<(u16, u16)>>,
	scroll_top: Cell<usize>,
	theme: SharedTheme,
	queue: Queue,
	key_config: SharedKeyConfig,
}

impl CommitList {
	///
	pub fn new(
		repo: RepoPathRef,
		title: &str,
		theme: SharedTheme,
		queue: Queue,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			repo,
			items: ItemBatch::default(),
			marked: Vec::with_capacity(2),
			selection: 0,
			count_total: 0,
			scroll_state: (Instant::now(), 0_f32),
			tags: None,
			local_branches: BTreeMap::default(),
			remote_branches: BTreeMap::default(),
			current_size: Cell::new(None),
			scroll_top: Cell::new(0),
			theme,
			queue,
			key_config,
			title: title.into(),
		}
	}

	///
	pub fn items(&mut self) -> &mut ItemBatch {
		&mut self.items
	}

	///
	pub const fn selection(&self) -> usize {
		self.selection
	}

	/// will return view size or None before the first render
	pub fn current_size(&self) -> Option<(u16, u16)> {
		self.current_size.get()
	}

	///
	pub fn set_count_total(&mut self, total: usize) {
		self.count_total = total;
		self.selection =
			cmp::min(self.selection, self.selection_max());
	}

	///
	#[allow(clippy::missing_const_for_fn)]
	pub fn selection_max(&self) -> usize {
		self.count_total.saturating_sub(1)
	}

	///
	pub const fn tags(&self) -> Option<&Tags> {
		self.tags.as_ref()
	}

	///
	pub fn clear(&mut self) {
		self.items.clear();
	}

	///
	pub fn set_tags(&mut self, tags: Tags) {
		self.tags = Some(tags);
	}

	///
	pub fn selected_entry(&self) -> Option<&LogEntry> {
		self.items.iter().nth(
			self.selection.saturating_sub(self.items.index_offset()),
		)
	}

	///
	pub fn selected_entry_marked(&self) -> bool {
		self.selected_entry()
			.and_then(|e| self.is_marked(&e.id))
			.unwrap_or_default()
	}

	///
	pub fn marked_count(&self) -> usize {
		self.marked.len()
	}

	///
	pub fn marked(&self) -> &[(usize, CommitId)] {
		&self.marked
	}

	///
	pub fn clear_marked(&mut self) {
		self.marked.clear();
	}

	///
	pub fn marked_commits(&self) -> Vec<CommitId> {
		let (_, commits): (Vec<_>, Vec<CommitId>) =
			self.marked.iter().copied().unzip();

		commits
	}

	pub fn copy_commit_hash(&self) -> Result<()> {
		let marked = self.marked.as_slice();
		let yank: Option<Cow<str>> = match marked {
			[] => self
				.items
				.iter()
				.nth(
					self.selection
						.saturating_sub(self.items.index_offset()),
				)
				.map(|e| Cow::Borrowed(e.hash_short.as_ref())),
			[(_idx, commit)] => {
				Some(commit.get_short_string().into())
			}
			[first, .., last] => {
				let marked_consecutive =
					marked.windows(2).all(|w| w[0].0 + 1 == w[1].0);

				let yank = if marked_consecutive {
					format!(
						"{}^..{}",
						first.1.get_short_string(),
						last.1.get_short_string()
					)
				} else {
					marked
						.iter()
						.map(|(_idx, commit)| {
							commit.get_short_string()
						})
						.join(" ")
				};
				Some(yank.into())
			}
		};

		if let Some(yank) = yank {
			crate::clipboard::copy_string(&yank)?;
			self.queue.push(InternalEvent::ShowInfoMsg(
				strings::copy_success(&yank),
			));
		}
		Ok(())
	}

	fn move_selection(&mut self, scroll: ScrollType) -> Result<bool> {
		self.update_scroll_speed();

		#[allow(clippy::cast_possible_truncation)]
		let speed_int = usize::try_from(self.scroll_state.1 as i64)?.max(1);

		let page_offset = usize::from(
			self.current_size.get().unwrap_or_default().1,
		)
		.saturating_sub(1);

		let new_selection = match scroll {
			ScrollType::Up => {
				self.selection.saturating_sub(speed_int)
			}
			ScrollType::Down => {
				self.selection.saturating_add(speed_int)
			}
			ScrollType::PageUp => {
				self.selection.saturating_sub(page_offset)
			}
			ScrollType::PageDown => {
				self.selection.saturating_add(page_offset)
			}
			ScrollType::Home => 0,
			ScrollType::End => self.selection_max(),
		};

		let new_selection =
			cmp::min(new_selection, self.selection_max());

		let needs_update = new_selection != self.selection;

		self.selection = new_selection;

		Ok(needs_update)
	}

	fn mark(&mut self) {
		if let Some(e) = self.selected_entry() {
			let id = e.id;
			let selected = self
				.selection
				.saturating_sub(self.items.index_offset());
			if self.is_marked(&id).unwrap_or_default() {
				self.marked.retain(|marked| marked.1 != id);
			} else {
				self.marked.push((selected, id));

				self.marked.sort_unstable_by(|first, second| {
					first.0.cmp(&second.0)
				});
			}
		}
	}

	fn update_scroll_speed(&mut self) {
		const REPEATED_SCROLL_THRESHOLD_MILLIS: u128 = 300;
		const SCROLL_SPEED_START: f32 = 0.1_f32;
		const SCROLL_SPEED_MAX: f32 = 10_f32;
		const SCROLL_SPEED_MULTIPLIER: f32 = 1.05_f32;

		let now = Instant::now();

		let since_last_scroll =
			now.duration_since(self.scroll_state.0);

		self.scroll_state.0 = now;

		let speed = if since_last_scroll.as_millis()
			< REPEATED_SCROLL_THRESHOLD_MILLIS
		{
			self.scroll_state.1 * SCROLL_SPEED_MULTIPLIER
		} else {
			SCROLL_SPEED_START
		};

		self.scroll_state.1 = speed.min(SCROLL_SPEED_MAX);
	}

	fn is_marked(&self, id: &CommitId) -> Option<bool> {
		if self.marked.is_empty() {
			None
		} else {
			let found =
				self.marked.iter().any(|entry| entry.1 == *id);
			Some(found)
		}
	}

	#[allow(clippy::too_many_arguments)]
	fn get_entry_to_add<'a>(
		e: &'a LogEntry,
		selected: bool,
		tags: Option<String>,
		local_branches: Option<String>,
		remote_branches: Option<String>,
		theme: &Theme,
		width: usize,
		now: DateTime<Local>,
		marked: Option<bool>,
	) -> Spans<'a> {
		let mut txt: Vec<Span> = Vec::with_capacity(
			ELEMENTS_PER_LINE + if marked.is_some() { 2 } else { 0 },
		);

		let splitter_txt = Cow::from(symbol::EMPTY_SPACE);
		let splitter =
			Span::styled(splitter_txt, theme.text(true, selected));

		// marker
		if let Some(marked) = marked {
			txt.push(Span::styled(
				Cow::from(if marked {
					symbol::CHECKMARK
				} else {
					symbol::EMPTY_SPACE
				}),
				theme.log_marker(selected),
			));
			txt.push(splitter.clone());
		}

		// commit hash
		txt.push(Span::styled(
			Cow::from(&*e.hash_short),
			theme.commit_hash(selected),
		));

		txt.push(splitter.clone());

		// commit timestamp
		txt.push(Span::styled(
			Cow::from(e.time_to_string(now)),
			theme.commit_time(selected),
		));

		txt.push(splitter.clone());

		let author_width =
			(width.saturating_sub(19) / 3).clamp(3, 20);
		let author = string_width_align(&e.author, author_width);

		// commit author
		txt.push(Span::styled::<String>(
			author,
			theme.commit_author(selected),
		));

		txt.push(splitter.clone());

		// commit tags
		if let Some(tags) = tags {
			txt.push(splitter.clone());
			txt.push(Span::styled(tags, theme.tags(selected)));
		}

		if let Some(local_branches) = local_branches {
			txt.push(splitter.clone());
			txt.push(Span::styled(
				local_branches,
				theme.branch(selected, true),
			));
		}

		if let Some(remote_branches) = remote_branches {
			txt.push(splitter.clone());
			txt.push(Span::styled(
				remote_branches,
				theme.branch(selected, true),
			));
		}

		txt.push(splitter);

		let message_width = width.saturating_sub(
			txt.iter().map(|span| span.content.len()).sum(),
		);

		// commit msg
		txt.push(Span::styled(
			format!("{:message_width$}", &e.msg),
			theme.text(true, selected),
		));

		Spans::from(txt)
	}

	fn get_text(&self, height: usize, width: usize) -> Vec<Spans> {
		let selection = self.relative_selection();

		let mut txt: Vec<Spans> = Vec::with_capacity(height);

		let now = Local::now();

		let any_marked = !self.marked.is_empty();

		for (idx, e) in self
			.items
			.iter()
			.skip(self.scroll_top.get())
			.take(height)
			.enumerate()
		{
			let tags =
				self.tags.as_ref().and_then(|t| t.get(&e.id)).map(
					|tags| {
						tags.iter()
							.map(|t| format!("<{}>", t.name))
							.join(" ")
					},
				);

			let local_branches =
				self.local_branches.get(&e.id).map(|local_branch| {
					local_branch
						.iter()
						.map(|local_branch| {
							format!("{{{0}}}", local_branch.name)
						})
						.join(" ")
				});

			let remote_branches = self
				.remote_branches
				.get(&e.id)
				.and_then(|remote_branches| {
					let filtered_branches: Vec<_> = remote_branches
						.iter()
						.filter(|remote_branch| {
							self.local_branches
								.get(&e.id)
								.map_or(true, |local_branch| {
									local_branch.iter().any(
										|local_branch| {
											let has_corresponding_local_branch = match &local_branch.details {
												BranchDetails::Local(details) =>
													details
														.upstream
														.as_ref()
														.map_or(false, |upstream| upstream.reference == remote_branch.reference),
												BranchDetails::Remote(_) =>
														false,
											};

											!has_corresponding_local_branch
										},
									)
								})
						})
						.map(|remote_branch| {
							format!("[{0}]", remote_branch.name)
						})
						.collect();

					if filtered_branches.is_empty() {
						None
					} else {
						Some(filtered_branches.join(" "))
					}
				});

			let marked = if any_marked {
				self.is_marked(&e.id)
			} else {
				None
			};

			txt.push(Self::get_entry_to_add(
				e,
				idx + self.scroll_top.get() == selection,
				tags,
				local_branches,
				remote_branches,
				&self.theme,
				width,
				now,
				marked,
			));
		}

		txt
	}

	#[allow(clippy::missing_const_for_fn)]
	fn relative_selection(&self) -> usize {
		self.selection.saturating_sub(self.items.index_offset())
	}

	pub fn select_entry(&mut self, position: usize) {
		self.selection = position;
	}

	pub fn checkout(&mut self) {
		if let Some(commit_hash) =
			self.selected_entry().map(|entry| entry.id)
		{
			try_or_popup!(
				self,
				"failed to checkout commit:",
				checkout_commit(&self.repo.borrow(), commit_hash)
			);
		}
	}

	pub fn set_local_branches(
		&mut self,
		local_branches: Vec<BranchInfo>,
	) {
		self.local_branches.clear();

		for local_branch in local_branches {
			self.local_branches
				.entry(local_branch.top_commit)
				.or_default()
				.push(local_branch);
		}
	}

	pub fn set_remote_branches(
		&mut self,
		remote_branches: Vec<BranchInfo>,
	) {
		self.remote_branches.clear();

		for remote_branch in remote_branches {
			self.remote_branches
				.entry(remote_branch.top_commit)
				.or_default()
				.push(remote_branch);
		}
	}
}

impl DrawableComponent for CommitList {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		area: Rect,
	) -> Result<()> {
		let current_size = (
			area.width.saturating_sub(2),
			area.height.saturating_sub(2),
		);
		self.current_size.set(Some(current_size));

		let height_in_lines = current_size.1 as usize;
		let selection = self.relative_selection();

		self.scroll_top.set(calc_scroll_top(
			self.scroll_top.get(),
			height_in_lines,
			selection,
		));

		let title = format!(
			"{} {}/{}",
			self.title,
			self.count_total.saturating_sub(self.selection),
			self.count_total,
		);

		f.render_widget(
			Paragraph::new(
				self.get_text(
					height_in_lines,
					current_size.0 as usize,
				),
			)
			.block(
				Block::default()
					.borders(Borders::ALL)
					.title(Span::styled(
						title.as_str(),
						self.theme.title(true),
					))
					.border_style(self.theme.block(true)),
			)
			.alignment(Alignment::Left),
			area,
		);

		draw_scrollbar(
			f,
			area,
			&self.theme,
			self.count_total,
			self.selection,
			Orientation::Vertical,
		);

		Ok(())
	}
}

impl Component for CommitList {
	fn event(&mut self, ev: &Event) -> Result<EventState> {
		if let Event::Key(k) = ev {
			let selection_changed =
				if key_match(k, self.key_config.keys.move_up) {
					self.move_selection(ScrollType::Up)?
				} else if key_match(k, self.key_config.keys.move_down)
				{
					self.move_selection(ScrollType::Down)?
				} else if key_match(k, self.key_config.keys.shift_up)
					|| key_match(k, self.key_config.keys.home)
				{
					self.move_selection(ScrollType::Home)?
				} else if key_match(
					k,
					self.key_config.keys.shift_down,
				) || key_match(k, self.key_config.keys.end)
				{
					self.move_selection(ScrollType::End)?
				} else if key_match(k, self.key_config.keys.page_up) {
					self.move_selection(ScrollType::PageUp)?
				} else if key_match(k, self.key_config.keys.page_down)
				{
					self.move_selection(ScrollType::PageDown)?
				} else if key_match(
					k,
					self.key_config.keys.log_mark_commit,
				) {
					self.mark();
					true
				} else if key_match(
					k,
					self.key_config.keys.log_checkout_commit,
				) {
					self.checkout();
					true
				} else {
					false
				};
			return Ok(selection_changed.into());
		}

		Ok(EventState::NotConsumed)
	}

	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		_force_all: bool,
	) -> CommandBlocking {
		out.push(CommandInfo::new(
			strings::commands::scroll(&self.key_config),
			self.selected_entry().is_some(),
			true,
		));
		out.push(CommandInfo::new(
			strings::commands::commit_list_mark(
				&self.key_config,
				self.selected_entry_marked(),
			),
			true,
			true,
		));
		CommandBlocking::PassingOn
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_string_width_align() {
		assert_eq!(string_width_align("123", 3), "123");
		assert_eq!(string_width_align("123", 2), "..");
		assert_eq!(string_width_align("123", 3), "123");
		assert_eq!(string_width_align("12345", 6), "12345 ");
		assert_eq!(string_width_align("1234556", 4), "12..");
	}

	#[test]
	fn test_string_width_align_unicode() {
		assert_eq!(string_width_align("äste", 3), "ä..");
		assert_eq!(
			string_width_align("wüsten äste", 10),
			"wüsten ä.."
		);
		assert_eq!(
			string_width_align("Jon Grythe Stødle", 19),
			"Jon Grythe Stødle  "
		);
	}
}
