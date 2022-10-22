use anyhow::Result;
use asyncgit::sync::{RepoPathRef, WorkTree};
use crossterm::event::Event;
use tui::{backend::Backend, Frame, layout::{Rect, Alignment}, text::{Span, Spans}, style::Style, widgets::{Paragraph, Block, Borders}};
use std::{cell::Cell, time::Instant, cmp};

use crate::{ui::{style::SharedTheme, calc_scroll_top, draw_scrollbar}, strings, keys::{SharedKeyConfig, key_match}, components::ScrollType};

use super::{DrawableComponent, Component, EventState, CommandInfo, CommandBlocking};


pub struct WorkTreesComponent {
	title: Box<str>,
	repo: RepoPathRef,
	visible: bool,
    theme: SharedTheme,
    worktrees: Vec<WorkTree>,
	current_size: Cell<(u16, u16)>,
	scroll_top: Cell<usize>,
	selection: usize,
	count_total: usize,
    key_config: SharedKeyConfig,
	scroll_state: (Instant, f32),
}

impl WorkTreesComponent {
	///
	pub fn new(
        title: &str,
		repo: RepoPathRef,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
	) -> Self {
		Self {
			title: title.into(),
			repo,
			visible: false,
            theme,
            worktrees: Vec::new(),
			current_size: Cell::new((0, 0)),
			scroll_top: Cell::new(0),
			selection: 0,
			count_total: 0,
            key_config,
			scroll_state: (Instant::now(), 0_f32),
		}
	}

    pub fn set_worktrees(&mut self, worktrees: Vec<WorkTree>) -> Result<()> {
        self.worktrees = worktrees;
        self.set_count_total(self.worktrees.len());
        Ok(())
    }

	fn is_visible(&self) -> bool {
		self.visible
	}

	fn relative_selection(&self) -> usize {
        // TODO: Do Stuff
		self.selection.saturating_sub(0)
	}

	fn get_text(&self, height: usize, width: usize) -> Vec<Spans> {
        self.worktrees.iter().map(|w| {
            Spans::from(vec![
                Span::styled(w.name.clone(), self.theme.text(true, true)),
            ])
        }).collect::<Vec<Spans>>()
    }

	fn move_selection(&mut self, scroll: ScrollType) -> Result<bool> {
        log::trace!("wtf is a scroll {:?}", scroll);

		//#[allow(clippy::cast_possible_truncation)]
		let speed_int = usize::try_from(self.scroll_state.1 as i64)?.max(1);

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

        log::trace!("selection: {}", new_selection);
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

}

impl DrawableComponent for WorkTreesComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		area: Rect,
	) -> Result<()> {
        log::trace!("delete me later {:?}", self.repo);
        log::trace!("shut clippy up: {}", self.is_visible());
        let items = self.worktrees.iter().map(|w| Span::styled(w.name.clone(), Style::default()));
		let current_size = (
			area.width.saturating_sub(2),
			area.height.saturating_sub(2),
		);
		self.current_size.set(current_size);

		let height_in_lines = self.current_size.get().1 as usize;
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
		);

		Ok(())
	}
}

impl Component for WorkTreesComponent {
	fn event(&mut self, ev: &Event) -> Result<EventState> {
        log::trace!("selected worktree: {}", self.selection);
        if let Event::Key(k) = ev {
            log::trace!("key pressed: {:?}", k);
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
				} else {
					false
				};
            log::trace!("selection changed {}", selection_changed);
            return Ok(selection_changed.into())
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
}
