use super::{
	textinput::TextInputComponent, visibility_blocking,
	CommandBlocking, CommandInfo, Component, DrawableComponent,
	EventState,
};
use crate::{
	keys::{key_match, SharedKeyConfig},
	queue::{InternalEvent, NeedsUpdate, Queue},
	strings,
	ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::sync::{self, RepoPathRef};
use crossterm::event::Event;
use ratatui::{backend::Backend, layout::Rect, Frame};

pub struct RenameBranchComponent {
	repo: RepoPathRef,
	input: TextInputComponent,
	branch_ref: Option<String>,
	queue: Queue,
	key_config: SharedKeyConfig,
}

impl DrawableComponent for RenameBranchComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		rect: Rect,
	) -> Result<()> {
		self.input.draw(f, rect)?;

		Ok(())
	}
}

impl Component for RenameBranchComponent {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.is_visible() || force_all {
			self.input.commands(out, force_all);

			out.push(CommandInfo::new(
				strings::commands::rename_branch_confirm_msg(
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
					self.rename_branch();
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

impl RenameBranchComponent {
	///
	pub fn new(
		repo: RepoPathRef,
		queue: Queue,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			repo,
			queue,
			input: TextInputComponent::new(
				theme,
				key_config.clone(),
				&strings::rename_branch_popup_title(&key_config),
				&strings::rename_branch_popup_msg(&key_config),
				true,
			),
			branch_ref: None,
			key_config,
		}
	}

	///
	pub fn open(
		&mut self,
		branch_ref: String,
		cur_name: String,
	) -> Result<()> {
		self.branch_ref = None;
		self.branch_ref = Some(branch_ref);
		self.input.set_text(cur_name);
		self.show()?;

		Ok(())
	}

	///
	pub fn rename_branch(&mut self) {
		if let Some(br) = &self.branch_ref {
			let res = sync::rename_branch(
				&self.repo.borrow(),
				br,
				self.input.get_text(),
			);

			match res {
				Ok(_) => {
					self.queue.push(InternalEvent::Update(
						NeedsUpdate::ALL,
					));
					self.hide();
					self.queue.push(InternalEvent::SelectBranch);
				}
				Err(e) => {
					log::error!("create branch: {}", e,);
					self.queue.push(InternalEvent::ShowErrorMsg(
						format!("rename branch error:\n{e}",),
					));
				}
			}
		} else {
			log::error!("create branch: No branch selected");
			self.queue.push(InternalEvent::ShowErrorMsg(
				"rename branch error: No branch selected to rename"
					.to_string(),
			));
		}

		self.input.clear();
	}
}
