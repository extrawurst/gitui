use std::cell::RefCell;

use crate::{
	keys::{key_match, SharedKeyConfig},
	queue::{InternalEvent, Queue},
	strings,
	ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::sync::{CommitId, RepoPath};
use crossterm::event::Event;
use easy_cast::Cast;
use ratatui::{backend::Backend, widgets::Paragraph, Frame};

use super::{
	visibility_blocking, CommandBlocking, CommandInfo, Component,
	DrawableComponent, EventState, TextInputComponent,
};

pub struct JumpCommitShaPopup {
	queue: Queue,
	visible: bool,
	key_config: SharedKeyConfig,
	repo: RefCell<RepoPath>,
	theme: SharedTheme,
	input: TextInputComponent,
	commit_id: Option<CommitId>,
	error_msg: String,
}

impl JumpCommitShaPopup {
	pub fn new(
		queue: &Queue,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
		repo: RefCell<RepoPath>,
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
			repo,
			input,
			commit_id: None,
			error_msg: String::default(),
		}
	}

	pub fn open(&mut self) -> Result<()> {
		self.show()?;
		self.input.show()?;
		self.input.set_text(String::new());
		self.commit_id = None;
		self.error_msg.clear();

		Ok(())
	}

	fn validate(&mut self) {
		let path = self.repo.borrow();
		if let Ok(commit_id) =
			CommitId::from_revision(self.input.get_text(), &path)
		{
			self.commit_id = Some(commit_id);
			self.error_msg.clear();
		} else {
			self.commit_id = None;
			self.error_msg = strings::jump_to_commit_err();
		}
	}

	fn is_sha_valid(&self) -> bool {
		self.commit_id.is_some()
	}

	fn execute_confirm(&mut self) {
		self.hide();

		let commit_id = self.commit_id.expect("Commit id must have value here because it's already validated");
		self.queue.push(InternalEvent::JumpToCommit(commit_id));
	}

	fn draw_error<B: Backend>(&self, f: &mut Frame<B>) {
		if self.is_sha_valid() {
			return;
		}

		let msg_len: u16 = self.error_msg.len().cast();

		let err_paragraph = Paragraph::new(self.error_msg.as_str())
			.style(self.theme.text_danger());

		let mut rect = self.input.get_area();
		rect.y += rect.height.saturating_sub(1);
		rect.height = 1;
		let offset = rect.width.saturating_sub(msg_len + 1);
		rect.width = rect.width.saturating_sub(offset + 1);
		rect.x += offset;

		f.render_widget(err_paragraph, rect);
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
			self.draw_error(f);
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
				self.execute_confirm();
			} else if self.input.event(event)?.is_consumed() {
				self.validate();
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
