use crate::{
	keys::SharedKeyConfig,
	queue::{InternalEvent, Queue},
	ui::style::SharedTheme,
};
use anyhow::Result;

use super::{
	visibility_blocking, CommandBlocking, CommandInfo, Component,
	DrawableComponent, EventState, TextInputComponent,
};

pub struct JumpCommitShaPopup {
	queue: Queue,
	visible: bool,
	key_config: SharedKeyConfig,
	theme: SharedTheme,
	input: TextInputComponent,
}

impl JumpCommitShaPopup {
	pub fn new(
		queue: &Queue,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		let input = TextInputComponent::new(
			theme.clone(),
			key_config.clone(),
			"TODO SHA",
			"TODO default MSG",
			false,
		);

		Self {
			queue: queue.clone(),
			visible: false,
			theme,
			key_config,
			input,
		}
	}

	pub fn open(&mut self) -> Result<()> {
		let _ = self.queue;
		let _ = self.visible;
		let _ = self.key_config;
		let _ = self.theme;
		let _ = self.input;

		let _ = InternalEvent::JumpToCommit(String::default());

		Ok(())
	}
}

impl DrawableComponent for JumpCommitShaPopup {
	fn draw<B: ratatui::backend::Backend>(
		&self,
		f: &mut ratatui::Frame<B>,
		rect: ratatui::layout::Rect,
	) -> Result<()> {
		Ok(())
	}
}

impl Component for JumpCommitShaPopup {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		visibility_blocking(self)
	}

	fn event(
		&mut self,
		ev: &crossterm::event::Event,
	) -> Result<EventState> {
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
