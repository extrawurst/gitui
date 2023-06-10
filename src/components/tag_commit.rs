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
use asyncgit::sync::{self, CommitId, RepoPathRef};
use crossterm::event::Event;
use ratatui::{backend::Backend, layout::Rect, Frame};

enum Mode {
	Name,
	Annotation { tag_name: String },
}

pub struct TagCommitComponent {
	repo: RepoPathRef,
	mode: Mode,
	input: TextInputComponent,
	commit_id: Option<CommitId>,
	queue: Queue,
	key_config: SharedKeyConfig,
}

impl DrawableComponent for TagCommitComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		rect: Rect,
	) -> Result<()> {
		self.input.draw(f, rect)?;

		Ok(())
	}
}

impl Component for TagCommitComponent {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.is_visible() || force_all {
			self.input.commands(out, force_all);

			out.push(CommandInfo::new(
				strings::commands::tag_commit_confirm_msg(
					&self.key_config,
				),
				self.is_valid_tag(),
				true,
			));

			out.push(CommandInfo::new(
				strings::commands::tag_annotate_msg(&self.key_config),
				self.is_valid_tag(),
				matches!(self.mode, Mode::Name),
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
				if key_match(e, self.key_config.keys.enter)
					&& self.is_valid_tag()
				{
					self.tag();
				} else if key_match(
					e,
					self.key_config.keys.tag_annotate,
				) && self.is_valid_tag()
				{
					let tag_name: String =
						self.input.get_text().into();

					self.input.clear();
					self.input.set_title(
						strings::tag_popup_annotation_title(
							&tag_name,
						),
					);
					self.input.set_default_msg(
						strings::tag_popup_annotation_msg(),
					);
					self.mode = Mode::Annotation { tag_name };
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
		self.mode = Mode::Name;
		self.input.set_title(strings::tag_popup_name_title());
		self.input.set_default_msg(strings::tag_popup_name_msg());
		self.input.show()?;

		Ok(())
	}
}

impl TagCommitComponent {
	///
	pub fn new(
		repo: RepoPathRef,
		queue: Queue,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			queue,
			input: TextInputComponent::new(
				theme,
				key_config.clone(),
				&strings::tag_popup_name_title(),
				&strings::tag_popup_name_msg(),
				true,
			),
			commit_id: None,
			key_config,
			repo,
			mode: Mode::Name,
		}
	}

	///
	pub fn open(&mut self, id: CommitId) -> Result<()> {
		self.commit_id = Some(id);
		self.show()?;

		Ok(())
	}

	fn is_valid_tag(&self) -> bool {
		!self.input.get_text().is_empty()
	}

	fn tag_info(&self) -> (String, Option<String>) {
		match &self.mode {
			Mode::Name => (self.input.get_text().into(), None),
			Mode::Annotation { tag_name } => {
				(tag_name.clone(), Some(self.input.get_text().into()))
			}
		}
	}

	///
	pub fn tag(&mut self) {
		let (tag_name, tag_annotation) = self.tag_info();

		if let Some(commit_id) = self.commit_id {
			let result = sync::tag_commit(
				&self.repo.borrow(),
				&commit_id,
				&tag_name,
				tag_annotation.as_deref(),
			);
			match result {
				Ok(_) => {
					self.input.clear();
					self.hide();

					self.queue.push(InternalEvent::Update(
						NeedsUpdate::ALL,
					));
				}
				Err(e) => {
					// go back to tag name if something goes wrong
					self.input.set_text(tag_name);
					self.hide();

					log::error!("e: {}", e,);
					self.queue.push(InternalEvent::ShowErrorMsg(
						format!("tag error:\n{e}",),
					));
				}
			}
		}
	}
}
