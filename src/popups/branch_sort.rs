use anyhow::Result;
use crossterm::event::Event;
use ratatui::{
	layout::{Alignment, Margin, Rect},
	text::{Line, Span},
	widgets::{Block, Borders, Clear, Paragraph},
	Frame,
};

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
	key_config: SharedKeyConfig,
	theme: SharedTheme,
}

impl BranchSortPopup {
	///
	pub fn new(env: &Environment) -> Self {
		Self {
			queue: env.queue.clone(),
			visible: false,
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

	fn get_sort_key_lines(&self) -> Vec<Line> {
		let texts = [
			strings::sort_branch_by_name_msg(&self.key_config),
			strings::sort_branch_by_name_rev_msg(&self.key_config),
			strings::sort_branch_by_time_msg(&self.key_config),
			strings::sort_branch_by_time_rev_msg(&self.key_config),
			strings::sort_branch_by_author_msg(&self.key_config),
			strings::sort_branch_by_author_rev_msg(&self.key_config),
		];
		texts
			.iter()
			.map(|t| {
				Line::from(vec![Span::styled(
					t.clone(),
					self.theme.text(true, false),
				)])
			})
			.collect()
	}
}

impl DrawableComponent for BranchSortPopup {
	fn draw(&self, f: &mut Frame, area: Rect) -> Result<()> {
		if self.is_visible() {
			const MAX_SIZE: (u16, u16) = (50, 8);

			let mut area = ui::centered_rect_absolute(
				MAX_SIZE.0, MAX_SIZE.1, area,
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
				strings::commands::close_popup(&self.key_config),
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
				if key_match(key, self.key_config.keys.exit_popup) {
					self.hide();
				} else if key_match(
					key,
					self.key_config.keys.branch_sort_by_name,
				) {
					self.update_sort_key(
						BranchListSortBy::BranchNameAsc,
					);
					self.hide();
				} else if key_match(
					key,
					self.key_config.keys.branch_sort_by_name_rev,
				) {
					self.update_sort_key(
						BranchListSortBy::BranchNameDesc,
					);
					self.hide();
				} else if key_match(
					key,
					self.key_config.keys.branch_sort_by_time,
				) {
					self.update_sort_key(
						BranchListSortBy::LastCommitTimeDesc,
					);
					self.hide();
				} else if key_match(
					key,
					self.key_config.keys.branch_sort_by_time_rev,
				) {
					self.update_sort_key(
						BranchListSortBy::LastCommitTimeAsc,
					);
					self.hide();
				} else if key_match(
					key,
					self.key_config.keys.branch_sort_by_author,
				) {
					self.update_sort_key(
						BranchListSortBy::LastCommitAuthorAsc,
					);
					self.hide();
				} else if key_match(
					key,
					self.key_config.keys.branch_sort_by_author_rev,
				) {
					self.update_sort_key(
						BranchListSortBy::LastCommitAuthorDesc,
					);
					self.hide();
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
