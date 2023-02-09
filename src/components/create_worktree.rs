use super::{
	textinput::TextInputComponent, visibility_blocking,
	CommandBlocking, CommandInfo, Component, DrawableComponent,
	EventState,
};
use crate::{
	keys::{key_match, SharedKeyConfig},
	strings,
	ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::sync::{self, RepoPathRef};
use crossterm::event::Event;
use tui::{backend::Backend, layout::Rect, Frame};

pub struct CreateWorktreeComponent {
	repo: RepoPathRef,
	input: TextInputComponent,
	key_config: SharedKeyConfig,
}

impl DrawableComponent for CreateWorktreeComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		rect: Rect,
	) -> Result<()> {
		if self.is_visible() {
			self.input.draw(f, rect)?;
		}

		Ok(())
	}
}
impl Component for CreateWorktreeComponent {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.is_visible() || force_all {
			self.input.commands(out, force_all);
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
					self.create_worktree();
				}

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

impl CreateWorktreeComponent {
	///
	pub fn new(
		repo: RepoPathRef,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			input: TextInputComponent::new(
				theme.clone(),
				key_config.clone(),
				&strings::create_branch_popup_title(&key_config),
				&strings::create_branch_popup_msg(&key_config),
				true,
			),
			key_config,
			repo,
		}
	}

	///
	pub fn open(&mut self) -> Result<()> {
		self.show()?;

		Ok(())
	}

	///
	pub fn create_worktree(&mut self) {
		let res = sync::create_worktree(
			&self.repo.borrow(),
			self.input.get_text(),
		);

		self.input.clear();
		self.hide();

		match res {
			Ok(_) => {
				log::trace!("Worktree created");
			}
			Err(e) => {
				log::trace!("Worktree creation failed: {}", e);
			}
		}
		// match res {
		// 	Ok(_) => {
		// 		self.queue.push(InternalEvent::Update(
		// 			NeedsUpdate::ALL | NeedsUpdate::BRANCHES,
		// 		));
		// 	}
		// 	Err(e) => {
		// 		log::error!("create branch: {}", e,);
		// 		self.queue.push(InternalEvent::ShowErrorMsg(
		// 			format!("create branch error:\n{e}",),
		// 		));
		// 	}
		// }
		//
	}
}
