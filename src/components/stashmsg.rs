use super::{
	textinput::TextInputComponent, visibility_blocking,
	CommandBlocking, CommandInfo, Component, DrawableComponent,
	EventState,
};
use crate::{
	keys::{key_match, SharedKeyConfig},
	queue::{InternalEvent, NeedsUpdate, Queue},
	strings::{self},
	tabs::StashingOptions,
	ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{sync::RepoPathRef, AsyncGitNotification, AsyncStash};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use ratatui::{backend::Backend, layout::Rect, Frame};

pub struct StashMsgComponent {
	options: StashingOptions,
	input: TextInputComponent,
	git_stash: AsyncStash,
	queue: Queue,
	key_config: SharedKeyConfig,
}

impl DrawableComponent for StashMsgComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		rect: Rect,
	) -> Result<()> {
		self.input.draw(f, rect)?;

		Ok(())
	}
}

impl Component for StashMsgComponent {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.is_visible() || force_all {
			self.input.commands(out, force_all);

			out.push(CommandInfo::new(
				strings::commands::stashing_confirm_msg(
					&self.key_config,
				),
				true,
				true,
			));
		}

		visibility_blocking(self)
	}

	fn event(&mut self, ev: &Event) -> Result<EventState> {
		if self.is_visible() {
			if self.input.event(ev)?.is_consumed() {
				return Ok(EventState::Consumed);
			}

			if let Event::Key(e) = ev {
				if key_match(e, self.key_config.keys.enter) {
					self.input.disable();
					self.input.set_title(
						strings::stash_popup_stashing(
							&self.key_config,
						),
					);
					self.git_stash.stash_save(
						if self.input.get_text().is_empty() {
							None
						} else {
							Some(self.input.get_text())
						},
						self.options.stash_untracked,
						self.options.keep_index,
					)?;
				}

				// stop key event propagation
				return Ok(EventState::Consumed);
			}
		}
		Ok(EventState::NotConsumed)
	}

	fn is_visible(&self) -> bool {
		self.input.is_visible()
	}

	fn hide(&mut self) {
		self.input.hide();
	}

	fn show(&mut self) -> Result<()> {
		self.input.show()?;

		Ok(())
	}
}

impl StashMsgComponent {
	///
	pub fn new(
		repo: &RepoPathRef,
		queue: Queue,
		sender: &Sender<AsyncGitNotification>,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			options: StashingOptions::default(),
			queue,
			input: TextInputComponent::new(
				theme,
				key_config.clone(),
				&strings::stash_popup_title(&key_config),
				&strings::stash_popup_msg(&key_config),
				true,
			),
			key_config,
			git_stash: AsyncStash::new(
				repo.borrow().clone(),
				sender.clone(),
			),
		}
	}

	///
	pub fn options(&mut self, options: StashingOptions) {
		self.options = options;
	}

	///
	pub fn anything_pending(&self) -> bool {
		self.git_stash.is_pending()
	}

	///
	pub fn update_git(&mut self, ev: AsyncGitNotification) {
		if self.is_visible() && ev == AsyncGitNotification::Stash {
			self.input.enable();
			self.input.clear();
			self.hide();

			self.queue.push(InternalEvent::Update(NeedsUpdate::ALL));
		}
	}
}
