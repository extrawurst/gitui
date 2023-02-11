use anyhow::Result;
use asyncgit::sync::WorkTree;
use crossterm::event::Event;
use std::{cell::Cell, cmp, time::Instant};
use tui::{
	backend::Backend,
	layout::{Alignment, Rect},
	text::{Span, Spans},
	widgets::{Block, Borders, Paragraph},
	Frame,
};

use crate::{
	components::{utils::string_width_align, ScrollType},
	keys::{key_match, SharedKeyConfig},
	strings::{self, symbol},
	ui::{calc_scroll_top, draw_scrollbar, style::SharedTheme},
};

use super::{
	textinput::TextInputComponent, CommandBlocking, CommandInfo,
	Component, DrawableComponent, EventState,
};

pub struct WorkTreesComponent {
	title: Box<str>,
	theme: SharedTheme,
	worktrees: Vec<WorkTree>,
	current_size: Cell<(u16, u16)>,
	scroll_top: Cell<usize>,
	selection: usize,
	count_total: usize,
	key_config: SharedKeyConfig,
	scroll_state: (Instant, f32),
	input: TextInputComponent,
}

impl WorkTreesComponent {
	///
	pub fn new(
		title: &str,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			title: title.into(),
			theme: theme.clone(),
			worktrees: Vec::new(),
			current_size: Cell::new((0, 0)),
			scroll_top: Cell::new(0),
			selection: 0,
			count_total: 0,
			key_config: key_config.clone(),
			scroll_state: (Instant::now(), 0_f32),
			input: TextInputComponent::new(
				theme,
				key_config,
				&strings::tag_popup_name_title(),
				&strings::tag_popup_name_msg(),
				true,
			),
		}
	}

	pub fn set_worktrees(
		&mut self,
		worktrees: Vec<WorkTree>,
	) -> Result<()> {
		self.worktrees = worktrees;
		self.set_count_total(self.worktrees.len());
		Ok(())
	}

	fn get_entry_to_add(
		&self,
		wt: &WorkTree,
		selected: bool,
		width: usize,
	) -> Spans {
		let mut txt = Vec::new();
		txt.push(Span::styled(
			string_width_align(
				match wt.is_locked {
					true => symbol::LOCK,
					false => "",
				},
				2,
			),
			self.theme.worktree(wt.is_valid, selected),
		));
		txt.push(Span::styled(
			string_width_align(
				match wt.is_current {
					true => symbol::CHECKMARK,
					false => "",
				},
				2,
			),
			self.theme.worktree(wt.is_valid, selected),
		));
		txt.push(Span::styled(
			string_width_align(&wt.name.clone(), width),
			self.theme.worktree(wt.is_valid, selected),
		));
		Spans(txt)
	}

	fn get_text(&self, height: usize, width: usize) -> Vec<Spans> {
		let mut txt: Vec<Spans> = Vec::with_capacity(height);
		for (idx, e) in self
			.worktrees
			.iter()
			.skip(self.scroll_top.get())
			.take(height)
			.enumerate()
		{
			txt.push(
				self.get_entry_to_add(
					e,
					idx == self
						.selection
						.saturating_sub(self.scroll_top.get()),
					width,
				),
			);
		}
		txt
	}

	fn move_selection(&mut self, scroll: ScrollType) -> Result<bool> {
		self.update_scroll_speed();

		//#[allow(clippy::cast_possible_truncation)]
		let speed_int =
			usize::try_from(self.scroll_state.1 as i64)?.max(1);

		let page_offset =
			usize::from(self.current_size.get().1).saturating_sub(1);

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

	pub fn selection_max(&self) -> usize {
		self.count_total.saturating_sub(1)
	}

	pub fn set_count_total(&mut self, total: usize) {
		self.count_total = total;
		self.selection =
			cmp::min(self.selection, self.selection_max());
	}

	pub fn selected_worktree(&self) -> Option<&WorkTree> {
		self.worktrees.get(self.selection)
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
}

impl DrawableComponent for WorkTreesComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		area: Rect,
	) -> Result<()> {
		let current_size = (
			area.width.saturating_sub(2),
			area.height.saturating_sub(2),
		);
		self.current_size.set(current_size);

		let height_in_lines = self.current_size.get().1 as usize;

		self.scroll_top.set(calc_scroll_top(
			self.scroll_top.get(),
			height_in_lines,
			self.selection,
		));

		// Not sure if the count is really nessesary
		let title = format!(
			"{} {}/{}",
			self.title,
			self.selection.saturating_add(1),
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
		);

		Ok(())
	}
}

impl Component for WorkTreesComponent {
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
				} else if key_match(k, self.key_config.keys.edit_file)
				{
					self.show()?;
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
			true,
			true,
		));
		CommandBlocking::PassingOn
	}

	fn show(&mut self) -> Result<()> {
		self.input.set_title(strings::tag_popup_name_title());
		self.input.set_default_msg(strings::tag_popup_name_msg());
		self.input.show()?;

		Ok(())
	}
}
