use crate::{
	keys::{key_match, SharedKeyConfig},
	queue::{InternalEvent, Queue},
	strings,
	ui::style::SharedTheme,
};
use anyhow::Result;
use crossterm::event::Event;

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
			&strings::jump_to_commit_title(),
			&strings::jump_to_commit_msg(),
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
		self.show()?;
		self.input.show()?;
		self.input.set_text(String::new());

		Ok(())
	}

	fn is_sha_valid(&self) -> bool {
		//TODO: Validation should be scene for the users
		let _ = self.theme;
		true
	}

	fn exectue_confirm(&mut self) {
		self.hide();

		debug_assert!(self.is_sha_valid());

		let sha = self.input.get_text().trim();

		self.queue.push(InternalEvent::JumpToCommit(sha.into()));
	}
}

impl DrawableComponent for JumpCommitShaPopup {
	fn draw<B: ratatui::backend::Backend>(
		&self,
		f: &mut ratatui::Frame<B>,
		rect: ratatui::layout::Rect,
	) -> Result<()> {
		if self.is_visible() {
			self.input.draw(f, rect)?;
		}

		Ok(())
	}
}

impl Component for JumpCommitShaPopup {
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

			out.push(CommandInfo::new(
				strings::commands::confirm_action(&self.key_config),
				self.is_sha_valid(),
				self.visible,
			));
		}

		visibility_blocking(self)
	}

	fn event(
		&mut self,
		event: &crossterm::event::Event,
	) -> Result<EventState> {
		if !self.is_visible() {
			return Ok(EventState::NotConsumed);
		}

		if let Event::Key(key) = &event {
			if key_match(key, self.key_config.keys.exit_popup) {
				self.hide();
			} else if key_match(key, self.key_config.keys.enter)
				&& self.is_sha_valid()
			{
				self.exectue_confirm();
			} else {
				self.input.event(event)?;
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
