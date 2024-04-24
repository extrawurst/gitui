use anyhow::Result;
use crossterm::event::Event;
use ratatui::{
	layout::{Alignment, Margin, Rect},
	text::{Line, Span},
	widgets::{Block, Borders, Clear, Paragraph},
	Frame,
};
use strum::{EnumCount, IntoEnumIterator};

use crate::{
	app::Environment,
	components::{
		visibility_blocking, BranchListSortBy, CommandBlocking,
		CommandInfo, Component, DrawableComponent, EventState,
	},
	keys::{key_match, SharedKeyConfig},
	queue::{InternalEvent, Queue},
	strings,
	ui::{self, style::SharedTheme},
};

pub struct BranchSortPopup {
	queue: Queue,
	visible: bool,
	selection: BranchListSortBy,
	key_config: SharedKeyConfig,
	theme: SharedTheme,
}

impl BranchSortPopup {
	///
	pub fn new(env: &Environment) -> Self {
		Self {
			queue: env.queue.clone(),
			visible: false,
			selection: BranchListSortBy::BranchNameAsc,
			key_config: env.key_config.clone(),
			theme: env.theme.clone(),
		}
	}

	pub fn open(&mut self, sort_by: BranchListSortBy) -> Result<()> {
		self.show()?;
		self.update_sort_key(sort_by);
		Ok(())
	}

	fn update_sort_key(&mut self, sort_by: BranchListSortBy) {
		self.queue.push(InternalEvent::BranchListSort(sort_by));
	}

	fn move_selection(&mut self, up: bool) {
		let diff = if up {
			BranchListSortBy::COUNT.saturating_sub(1)
		} else {
			1
		};
		let new_selection = (self.selection as usize)
			.saturating_add(diff)
			.rem_euclid(BranchListSortBy::COUNT);
		self.selection = BranchListSortBy::iter()
			.collect::<Vec<BranchListSortBy>>()[new_selection];
	}

	fn get_sort_key_lines(&self) -> Vec<Line> {
		let texts = [
			strings::sort_branch_by_name_msg(
				self.selection.is_branch_name_asc(),
			),
			strings::sort_branch_by_name_rev_msg(
				self.selection.is_branch_name_desc(),
			),
			strings::sort_branch_by_time_msg(
				self.selection.is_last_commit_time_desc(),
			),
			strings::sort_branch_by_time_rev_msg(
				self.selection.is_last_commit_time_asc(),
			),
			strings::sort_branch_by_author_msg(
				self.selection.is_last_commit_author_asc(),
			),
			strings::sort_branch_by_author_rev_msg(
				self.selection.is_last_commit_author_desc(),
			),
		];
		texts
			.iter()
			.map(|t| {
				Line::from(vec![Span::styled(
					t.clone(),
					self.theme.text(t.starts_with("[X]"), false),
				)])
			})
			.collect()
	}
}

impl DrawableComponent for BranchSortPopup {
	fn draw(&self, f: &mut Frame, area: Rect) -> Result<()> {
		if self.is_visible() {
			let height = u16::try_from(BranchListSortBy::COUNT)?
				.saturating_add(2);
			let max_size: (u16, u16) = (50, height);

			let mut area = ui::centered_rect_absolute(
				max_size.0, max_size.1, area,
			);

			f.render_widget(Clear, area);
			f.render_widget(
				Block::default()
					.borders(Borders::all())
					.style(self.theme.title(true))
					.title(Span::styled(
						strings::POPUP_TITLE_BRANCH_SORT,
						self.theme.title(true),
					)),
				area,
			);

			area = area.inner(&Margin {
				horizontal: 1,
				vertical: 1,
			});
			f.render_widget(
				Paragraph::new(self.get_sort_key_lines())
					.block(
						Block::default()
							.borders(Borders::NONE)
							.border_style(self.theme.block(true)),
					)
					.alignment(Alignment::Left),
				area,
			);
		}
		Ok(())
	}
}

impl Component for BranchSortPopup {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.is_visible() || force_all {
			out.push(CommandInfo::new(
				strings::commands::close_branch_sort_popup(
					&self.key_config,
				),
				true,
				true,
			));
			out.push(CommandInfo::new(
				strings::commands::scroll(&self.key_config),
				true,
				true,
			));
		}

		visibility_blocking(self)
	}

	fn event(
		&mut self,
		event: &crossterm::event::Event,
	) -> Result<EventState> {
		if self.is_visible() {
			if let Event::Key(key) = event {
				if key_match(key, self.key_config.keys.exit_popup)
					|| key_match(key, self.key_config.keys.enter)
				{
					self.hide();
				} else if key_match(key, self.key_config.keys.move_up)
				{
					self.move_selection(true);
					self.update_sort_key(self.selection);
				} else if key_match(
					key,
					self.key_config.keys.move_down,
				) {
					self.move_selection(false);
					self.update_sort_key(self.selection);
				}
			}
			return Ok(EventState::Consumed);
		}

		Ok(EventState::NotConsumed)
	}

	fn is_visible(&self) -> bool {
		self.visible
	}

	fn hide(&mut self) {
		self.visible = false;
	}

	fn show(&mut self) -> Result<()> {
		self.visible = true;
		Ok(())
	}
}
