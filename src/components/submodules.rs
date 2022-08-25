//TODO:
#![allow(dead_code)]

use super::{
	visibility_blocking, CommandBlocking, CommandInfo, Component,
	DrawableComponent, EventState,
};
use crate::{
	keys::{key_match, SharedKeyConfig},
	queue::Queue,
	strings,
	ui::{self, Size},
};
use anyhow::Result;
use asyncgit::sync::{get_submodules, RepoPathRef, SubmoduleInfo};
use crossterm::event::Event;
use std::{cell::Cell, convert::TryInto};
use tui::{
	backend::Backend,
	layout::{Alignment, Margin, Rect},
	text::{Span, Spans, Text},
	widgets::{Block, BorderType, Borders, Clear, Paragraph},
	Frame,
};
use ui::style::SharedTheme;
use unicode_truncate::UnicodeTruncateStr;

///
pub struct SubmodulesListComponent {
	repo: RepoPathRef,
	submodules: Vec<SubmoduleInfo>,
	visible: bool,
	current_height: Cell<u16>,
	queue: Queue,
	theme: SharedTheme,
	key_config: SharedKeyConfig,
}

impl DrawableComponent for SubmodulesListComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		rect: Rect,
	) -> Result<()> {
		if self.is_visible() {
			const PERCENT_SIZE: Size = Size::new(80, 50);
			const MIN_SIZE: Size = Size::new(60, 20);

			let area = ui::centered_rect(
				PERCENT_SIZE.width,
				PERCENT_SIZE.height,
				f.size(),
			);
			let area =
				ui::rect_inside(MIN_SIZE, f.size().into(), area);
			let area = area.intersection(rect);

			f.render_widget(Clear, area);

			f.render_widget(
				Block::default()
					.title(strings::title_branches())
					.border_type(BorderType::Thick)
					.borders(Borders::ALL),
				area,
			);

			let area = area.inner(&Margin {
				vertical: 1,
				horizontal: 1,
			});

			// let chunks = Layout::default()
			// 	.direction(Direction::Vertical)
			// 	.constraints(
			// 		[Constraint::Length(2), Constraint::Min(1)]
			// 			.as_ref(),
			// 	)
			// 	.split(area);

			self.draw_list(f, area)?;
		}

		Ok(())
	}
}

impl Component for SubmodulesListComponent {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.visible || force_all {
			if !force_all {
				out.clear();
			}

			out.push(CommandInfo::new(
				strings::commands::scroll(&self.key_config),
				true,
				true,
			));

			out.push(CommandInfo::new(
				strings::commands::close_popup(&self.key_config),
				true,
				true,
			));

			//TODO: update submodules
		}
		visibility_blocking(self)
	}

	fn event(&mut self, ev: &Event) -> Result<EventState> {
		if !self.visible {
			return Ok(EventState::NotConsumed);
		}

		if let Event::Key(e) = ev {
			if key_match(e, self.key_config.keys.exit_popup) {
				self.hide();
			// } else if key_match(e, self.key_config.keys.move_down) {
			// 	return self
			// 		.move_selection(ScrollType::Up)
			// 		.map(Into::into);
			// } else if key_match(e, self.key_config.keys.move_up) {
			// 	return self
			// 		.move_selection(ScrollType::Down)
			// 		.map(Into::into);
			// } else if key_match(e, self.key_config.keys.page_down) {
			// 	return self
			// 		.move_selection(ScrollType::PageDown)
			// 		.map(Into::into);
			// } else if key_match(e, self.key_config.keys.page_up) {
			// 	return self
			// 		.move_selection(ScrollType::PageUp)
			// 		.map(Into::into);
			// } else if key_match(e, self.key_config.keys.home) {
			// 	return self
			// 		.move_selection(ScrollType::Home)
			// 		.map(Into::into);
			// } else if key_match(e, self.key_config.keys.end) {
			// 	return self
			// 		.move_selection(ScrollType::End)
			// 		.map(Into::into);
			} else if key_match(
				e,
				self.key_config.keys.cmd_bar_toggle,
			) {
				//do not consume if its the more key
				return Ok(EventState::NotConsumed);
			}
		}

		Ok(EventState::Consumed)
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

impl SubmodulesListComponent {
	pub fn new(
		repo: RepoPathRef,
		queue: Queue,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			submodules: Vec::new(),
			visible: false,
			queue,
			theme,
			key_config,
			current_height: Cell::new(0),
			repo,
		}
	}

	///
	pub fn open(&mut self) -> Result<()> {
		self.show()?;
		self.update_submodules()?;

		Ok(())
	}

	///
	pub fn update_submodules(&mut self) -> Result<()> {
		if self.is_visible() {
			self.submodules = get_submodules(&self.repo.borrow())?;

			// self.set_selection(self.selection)?;
		}
		Ok(())
	}

	fn valid_selection(&self) -> bool {
		!self.submodules.is_empty()
	}

	fn get_text(
		&self,
		theme: &SharedTheme,
		width_available: u16,
		height: usize,
	) -> Text {
		const UPSTREAM_SYMBOL: char = '\u{2191}';
		const TRACKING_SYMBOL: char = '\u{2193}';
		const HEAD_SYMBOL: char = '*';
		const EMPTY_SYMBOL: char = ' ';
		const THREE_DOTS: &str = "...";
		const COMMIT_HASH_LENGTH: usize = 8;
		const IS_HEAD_STAR_LENGTH: usize = 3; // "*  "
		const THREE_DOTS_LENGTH: usize = THREE_DOTS.len(); // "..."

		let branch_name_length: usize =
			width_available as usize * 40 / 100;
		// commit message takes up the remaining width
		let _commit_message_length: usize = (width_available
			as usize)
			.saturating_sub(COMMIT_HASH_LENGTH)
			.saturating_sub(branch_name_length)
			.saturating_sub(IS_HEAD_STAR_LENGTH)
			.saturating_sub(THREE_DOTS_LENGTH);
		let mut txt = Vec::new();

		for (i, displaybranch) in self
			.submodules
			.iter()
			// .skip(self.scroll.get_top())
			.take(height)
			.enumerate()
		{
			let mut module_path = displaybranch
				.path
				.as_os_str()
				.to_string_lossy()
				.to_string();
			if module_path.len()
				> branch_name_length.saturating_sub(THREE_DOTS_LENGTH)
			{
				module_path = module_path
					.unicode_truncate(
						branch_name_length
							.saturating_sub(THREE_DOTS_LENGTH),
					)
					.0
					.to_string();
				module_path += THREE_DOTS;
			}

			// let selected = (self.selection as usize
			// 	- self.scroll.get_top())
			// 	== i;
			let selected = false;

			let span_hash = Span::styled(
				format!(
					"{} ",
					displaybranch
						.head_id
						.unwrap_or_default()
						.get_short_string()
				),
				theme.commit_hash(selected),
			);

			let span_name = Span::styled(
				format!(
					"{:w$} ",
					module_path,
					w = branch_name_length
				),
				theme.branch(selected, true),
			);

			txt.push(Spans::from(vec![span_name, span_hash]));
		}

		Text::from(txt)
	}

	fn draw_list<B: Backend>(
		&self,
		f: &mut Frame<B>,
		r: Rect,
	) -> Result<()> {
		let height_in_lines = r.height as usize;
		self.current_height.set(height_in_lines.try_into()?);

		// self.scroll.update(
		// 	self.selection as usize,
		// 	self.submodules.len(),
		// 	height_in_lines,
		// );

		f.render_widget(
			Paragraph::new(self.get_text(
				&self.theme,
				r.width,
				height_in_lines,
			))
			.alignment(Alignment::Left),
			r,
		);

		let mut r = r;
		r.width += 1;
		r.height += 2;
		r.y = r.y.saturating_sub(1);

		// self.scroll.draw(f, r, &self.theme);

		Ok(())
	}
}
