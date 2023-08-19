use crate::{
	keys::SharedKeyConfig, queue::Queue, ui::style::SharedTheme,
};

use super::{
	visibility_blocking, CommandInfo, Component, DrawableComponent,
	EventState,
};

pub struct JumpCommitShaPopup {
	queue: Queue,
	visible: bool,
	key_config: SharedKeyConfig,
	theme: SharedTheme,
}

impl JumpCommitShaPopup {
	pub fn new(
		queue: &Queue,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			queue: queue.clone(),
			visible: false,
			theme,
			key_config,
		}
	}

	pub fn open(&mut self) -> anyhow::Result<()> {
		let _ = self.queue;
		let _ = self.visible;
		let _ = self.key_config;
		let _ = self.theme;

		Ok(())
	}
}

impl DrawableComponent for JumpCommitShaPopup {
	fn draw<B: ratatui::backend::Backend>(
		&self,
		f: &mut ratatui::Frame<B>,
		rect: ratatui::layout::Rect,
	) -> anyhow::Result<()> {
		Ok(())
	}
}

impl Component for JumpCommitShaPopup {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> super::CommandBlocking {
		visibility_blocking(self)
	}

	fn event(
		&mut self,
		ev: &crossterm::event::Event,
	) -> anyhow::Result<EventState> {
		Ok(EventState::NotConsumed)
	}
}
