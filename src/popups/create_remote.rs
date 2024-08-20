use std::borrow::Borrow;

use anyhow::Result;
use asyncgit::sync::{add_remote, validate_remote_name, RepoPathRef};
use crossterm::event::Event;
use easy_cast::Cast;
use ratatui::{widgets::Paragraph, Frame};

use crate::{
	app::Environment,
	components::{
		visibility_blocking, CommandBlocking, CommandInfo, Component,
		DrawableComponent, EventState, InputType, TextInputComponent,
	},
	keys::{key_match, SharedKeyConfig},
	queue::{InternalEvent, NeedsUpdate, Queue},
	strings,
	ui::style::SharedTheme,
};

pub struct CreateRemotePopup {
	repo: RepoPathRef,
	input: TextInputComponent,
	queue: Queue,
	key_config: SharedKeyConfig,
	provided_name: Option<String>,
	theme: SharedTheme,
}

impl DrawableComponent for CreateRemotePopup {
	fn draw(
		&self,
		f: &mut ratatui::Frame,
		rect: ratatui::prelude::Rect,
	) -> anyhow::Result<()> {
		if self.is_visible() {
			self.input.draw(f, rect)?;
			self.draw_warnings(f);
		}
		Ok(())
	}
}

impl Component for CreateRemotePopup {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.is_visible() || force_all {
			self.input.commands(out, force_all);

			out.push(CommandInfo::new(
				strings::commands::remote_confirm_name_msg(
					&self.key_config,
				),
				true,
				true,
			));
		}
		visibility_blocking(self)
	}

	fn event(
		&mut self,
		ev: &crossterm::event::Event,
	) -> Result<EventState> {
		if self.is_visible() {
			if self.input.event(ev)?.is_consumed() {
				return Ok(EventState::Consumed);
			}

			if let Event::Key(e) = ev {
				if key_match(e, self.key_config.keys.enter) {
					self.handle_submit()?;
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

impl CreateRemotePopup {
	pub fn new(env: &Environment) -> Self {
		Self {
			repo: env.repo.clone(),
			queue: env.queue.clone(),
			input: TextInputComponent::new(
				env,
				&strings::create_remote_popup_title_name(
					&env.key_config,
				),
				&strings::create_remote_popup_msg_name(
					&env.key_config,
				),
				true,
			)
			.with_input_type(InputType::Singleline),
			key_config: env.key_config.clone(),
			provided_name: None,
			theme: env.theme.clone(),
		}
	}

	pub fn open(&mut self) -> Result<()> {
		self.show()?;

		Ok(())
	}

	fn draw_warnings(&self, f: &mut Frame) {
		let current_text = self.input.get_text();

		if !current_text.is_empty() {
			let valid = if self.provided_name.is_none() {
				validate_remote_name(current_text)
			} else {
				true
			};

			if !valid {
				let msg = strings::remote_name_invalid();
				let msg_length: u16 = msg.len().cast();
				let w = Paragraph::new(msg)
					.style(self.theme.text_danger());

				let rect = {
					let mut rect = self.input.get_area();
					rect.y += rect.height.saturating_sub(1);
					rect.height = 1;
					let offset =
						rect.width.saturating_sub(msg_length + 1);
					rect.width =
						rect.width.saturating_sub(offset + 1);
					rect.x += offset;

					rect
				};

				f.render_widget(w, rect);
			}
		}
	}

	fn handle_submit(&mut self) -> Result<()> {
		if let Some(name) = self.provided_name.borrow() {
			let res = add_remote(
				&self.repo.borrow(),
				name,
				self.input.get_text(),
			);
			match res {
				Ok(()) => {
					self.queue.push(InternalEvent::Update(
						NeedsUpdate::ALL | NeedsUpdate::REMOTES,
					));
				}
				Err(e) => {
					log::error!("create remote: {}", e,);
					self.queue.push(InternalEvent::ShowErrorMsg(
						format!("create remote error:\n{e}",),
					));
				}
			}
			self.provided_name = None;
			self.input.clear();
			self.input.set_title(
				strings::create_remote_popup_title_name(
					&self.key_config,
				),
			);
			self.input.set_default_msg(
				strings::create_remote_popup_msg_name(
					&self.key_config,
				),
			);
			self.hide();
		} else {
			self.provided_name =
				Some(self.input.get_text().to_string());
			self.hide();
			self.input.clear();
			self.input.set_title(
				strings::create_remote_popup_title_url(
					&self.key_config,
				),
			);
			self.input.set_default_msg(
				strings::create_remote_popup_msg_url(
					&self.key_config,
				),
			);
			self.show()?;
		}
		Ok(())
	}
}
