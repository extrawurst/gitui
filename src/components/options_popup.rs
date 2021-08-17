#![allow(dead_code)]

use std::{cell::RefCell, rc::Rc};

use super::{
	visibility_blocking, CommandBlocking, CommandInfo, Component,
	DrawableComponent, EventState,
};
use crate::{
	keys::SharedKeyConfig,
	strings::{self},
};
use anyhow::Result;
use asyncgit::sync::ShowUntrackedFilesConfig;
use crossterm::event::Event;
use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};

#[derive(Default, Copy, Clone)]
pub struct Options {
	pub status_show_untracked: Option<ShowUntrackedFilesConfig>,
	pub diff_ignore_whitespaces: bool,
	pub diff_context_lines: i32,
	pub diff_interhunk_lines: i32,
}

pub type SharedOptions = Rc<RefCell<Options>>;

pub struct OptionsPopupComponent {
	visible: bool,
	key_config: SharedKeyConfig,
	options: SharedOptions,
}

impl OptionsPopupComponent {
	///
	pub fn new(
		key_config: SharedKeyConfig,
		options: SharedOptions,
	) -> Self {
		Self {
			visible: false,
			key_config,
			options,
		}
	}
}

impl DrawableComponent for OptionsPopupComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		area: Rect,
	) -> Result<()> {
		if self.is_visible() {
			f.render_widget(Clear, area);
		}

		Ok(())
	}
}

impl Component for OptionsPopupComponent {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.is_visible() || force_all {
			out.push(
				CommandInfo::new(
					strings::commands::close_popup(&self.key_config),
					true,
					true,
				)
				.order(1),
			);
		}

		visibility_blocking(self)
	}

	fn event(
		&mut self,
		event: crossterm::event::Event,
	) -> Result<EventState> {
		if self.is_visible() {
			if let Event::Key(key) = &event {
				if *key == self.key_config.exit_popup {
					self.hide();

					return Ok(EventState::Consumed);
				}
			}
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
