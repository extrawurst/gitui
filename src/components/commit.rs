use super::{
	textinput::TextInputComponent, visibility_blocking,
	CommandBlocking, CommandInfo, Component, DrawableComponent,
	EventState, ExternalEditorComponent,
};
use crate::{
	keys::{key_match, SharedKeyConfig},
	options::SharedOptions,
	queue::{InternalEvent, NeedsUpdate, Queue},
	strings, try_or_popup,
	ui::style::SharedTheme,
};
use anyhow::{bail, Ok, Result};
use asyncgit::{
	cached, message_prettify,
	sync::{
		self, get_config_string, CommitId, HookResult,
		PrepareCommitMsgSource, RepoPathRef, RepoState,
	},
	StatusItem, StatusItemType,
};
use crossterm::event::Event;
use easy_cast::Cast;
use ratatui::{
	backend::Backend,
	layout::{Alignment, Rect},
	widgets::Paragraph,
	Frame,
};
use std::{
	fs::{read_to_string, File},
	io::{Read, Write},
	path::PathBuf,
	str::FromStr,
};

enum CommitResult {
	ComitDone,
	Aborted,
}

enum Mode {
	Normal,
	Amend(CommitId),
	Merge(Vec<CommitId>),
	Revert,
	Reword(CommitId),
}

pub struct CommitComponent {
	repo: RepoPathRef,
	input: TextInputComponent,
	mode: Mode,
	queue: Queue,
	key_config: SharedKeyConfig,
	git_branch_name: cached::BranchName,
	commit_template: Option<String>,
	theme: SharedTheme,
	commit_msg_history_idx: usize,
	options: SharedOptions,
	verify: bool,
}

const FIRST_LINE_LIMIT: usize = 50;

impl CommitComponent {
	///
	pub fn new(
		repo: RepoPathRef,
		queue: Queue,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
		options: SharedOptions,
	) -> Self {
		Self {
			queue,
			mode: Mode::Normal,
			input: TextInputComponent::new(
				theme.clone(),
				key_config.clone(),
				"",
				&strings::commit_msg(&key_config),
				true,
			),
			key_config,
			git_branch_name: cached::BranchName::new(repo.clone()),
			commit_template: None,
			theme,
			repo,
			commit_msg_history_idx: 0,
			options,
			verify: true,
		}
	}

	///
	pub fn update(&mut self) {
		self.git_branch_name.lookup().ok();
	}

	fn draw_branch_name<B: Backend>(&self, f: &mut Frame<B>) {
		if let Some(name) = self.git_branch_name.last() {
			let w = Paragraph::new(format!("{{{name}}}"))
				.alignment(Alignment::Right);

			let rect = {
				let mut rect = self.input.get_area();
				rect.height = 1;
				rect.width = rect.width.saturating_sub(1);
				rect
			};

			f.render_widget(w, rect);
		}
	}

	fn draw_warnings<B: Backend>(&self, f: &mut Frame<B>) {
		let first_line = self
			.input
			.get_text()
			.lines()
			.next()
			.map(str::len)
			.unwrap_or_default();

		if first_line > FIRST_LINE_LIMIT {
			let msg = strings::commit_first_line_warning(first_line);
			let msg_length: u16 = msg.len().cast();
			let w =
				Paragraph::new(msg).style(self.theme.text_danger());

			let rect = {
				let mut rect = self.input.get_area();
				rect.y += rect.height.saturating_sub(1);
				rect.height = 1;
				let offset =
					rect.width.saturating_sub(msg_length + 1);
				rect.width = rect.width.saturating_sub(offset + 1);
				rect.x += offset;

				rect
			};

			f.render_widget(w, rect);
		}
	}

	const fn item_status_char(
		item_type: StatusItemType,
	) -> &'static str {
		match item_type {
			StatusItemType::Modified => "modified",
			StatusItemType::New => "new file",
			StatusItemType::Deleted => "deleted",
			StatusItemType::Renamed => "renamed",
			StatusItemType::Typechange => " ",
			StatusItemType::Conflicted => "conflicted",
		}
	}

	pub fn show_editor(
		&mut self,
		changes: Vec<StatusItem>,
	) -> Result<()> {
		let file_path = sync::repo_dir(&self.repo.borrow())?
			.join("COMMIT_EDITMSG");

		{
			let mut file = File::create(&file_path)?;
			file.write_fmt(format_args!(
				"{}\n",
				self.input.get_text()
			))?;
			file.write_all(
				strings::commit_editor_msg(&self.key_config)
					.as_bytes(),
			)?;

			file.write_all(b"\n#\n# Changes to be commited:")?;

			for change in changes {
				let status_char =
					Self::item_status_char(change.status);
				let message =
					format!("\n#\t{status_char}: {}", change.path);
				file.write_all(message.as_bytes())?;
			}
		}

		ExternalEditorComponent::open_file_in_editor(
			&self.repo.borrow(),
			&file_path,
		)?;

		let mut message = String::new();

		let mut file = File::open(&file_path)?;
		file.read_to_string(&mut message)?;
		drop(file);
		std::fs::remove_file(&file_path)?;

		message = message_prettify(message, Some(b'#'))?;
		self.input.set_text(message);
		self.input.show()?;

		Ok(())
	}

	fn commit(&mut self) -> Result<()> {
		let gpgsign =
			get_config_string(&self.repo.borrow(), "commit.gpgsign")
				.ok()
				.flatten()
				.and_then(|path| path.parse::<bool>().ok())
				.unwrap_or_default();

		if gpgsign {
			anyhow::bail!("config commit.gpgsign=true detected.\ngpg signing not supported.\ndeactivate in your repo/gitconfig to be able to commit without signing.");
		}

		let msg = self.input.get_text().to_string();

		if matches!(
			self.commit_with_msg(msg)?,
			CommitResult::ComitDone
		) {
			self.options
				.borrow_mut()
				.add_commit_msg(self.input.get_text());
			self.commit_msg_history_idx = 0;

			self.hide();
			self.queue.push(InternalEvent::Update(NeedsUpdate::ALL));
			self.input.clear();
		}

		Ok(())
	}

	fn commit_with_msg(
		&mut self,
		msg: String,
	) -> Result<CommitResult> {
		// on exit verify should always be on
		let verify = self.verify;
		self.verify = true;

		if verify {
			// run pre commit hook - can reject commit
			if let HookResult::NotOk(e) =
				sync::hooks_pre_commit(&self.repo.borrow())?
			{
				log::error!("pre-commit hook error: {}", e);
				self.queue.push(InternalEvent::ShowErrorMsg(
					format!("pre-commit hook error:\n{e}"),
				));
				return Ok(CommitResult::Aborted);
			}
		}

		let mut msg = message_prettify(msg, Some(b'#'))?;

		if verify {
			// run commit message check hook - can reject commit
			if let HookResult::NotOk(e) =
				sync::hooks_commit_msg(&self.repo.borrow(), &mut msg)?
			{
				log::error!("commit-msg hook error: {}", e);
				self.queue.push(InternalEvent::ShowErrorMsg(
					format!("commit-msg hook error:\n{e}"),
				));
				return Ok(CommitResult::Aborted);
			}
		}
		self.do_commit(&msg)?;

		if let HookResult::NotOk(e) =
			sync::hooks_post_commit(&self.repo.borrow())?
		{
			log::error!("post-commit hook error: {}", e);
			self.queue.push(InternalEvent::ShowErrorMsg(format!(
				"post-commit hook error:\n{e}"
			)));
		}

		Ok(CommitResult::ComitDone)
	}

	fn do_commit(&self, msg: &str) -> Result<()> {
		match &self.mode {
			Mode::Normal => sync::commit(&self.repo.borrow(), msg)?,
			Mode::Amend(amend) => {
				sync::amend(&self.repo.borrow(), *amend, msg)?
			}
			Mode::Merge(ids) => {
				sync::merge_commit(&self.repo.borrow(), msg, ids)?
			}
			Mode::Revert => {
				sync::commit_revert(&self.repo.borrow(), msg)?
			}
			Mode::Reword(id) => {
				let commit =
					sync::reword(&self.repo.borrow(), *id, msg)?;
				self.queue.push(InternalEvent::TabSwitchStatus);

				commit
			}
		};
		Ok(())
	}

	fn can_commit(&self) -> bool {
		!self.is_empty() && self.is_changed()
	}

	fn can_amend(&self) -> bool {
		matches!(self.mode, Mode::Normal)
			&& sync::get_head(&self.repo.borrow()).is_ok()
			&& (self.is_empty() || !self.is_changed())
	}

	fn is_empty(&self) -> bool {
		self.input.get_text().is_empty()
	}

	fn is_changed(&self) -> bool {
		Some(self.input.get_text().trim())
			!= self.commit_template.as_ref().map(|s| s.trim())
	}

	fn amend(&mut self) -> Result<()> {
		if self.can_amend() {
			let id = sync::get_head(&self.repo.borrow())?;
			self.mode = Mode::Amend(id);

			let details =
				sync::get_commit_details(&self.repo.borrow(), id)?;

			self.input.set_title(strings::commit_title_amend());

			if let Some(msg) = details.message {
				self.input.set_text(msg.combine());
			}
		}

		Ok(())
	}
	fn signoff_commit(&mut self) {
		let msg = self.input.get_text();
		let signed_msg = self.add_sign_off(msg);
		if let std::result::Result::Ok(signed_msg) = signed_msg {
			self.input.set_text(signed_msg);
		}
	}
	fn toggle_verify(&mut self) {
		self.verify = !self.verify;
	}

	pub fn open(&mut self, reword: Option<CommitId>) -> Result<()> {
		//only clear text if it was not a normal commit dlg before, so to preserve old commit msg that was edited
		if !matches!(self.mode, Mode::Normal) {
			self.input.clear();
		}

		self.mode = Mode::Normal;

		let repo_state = sync::repo_state(&self.repo.borrow())?;

		let (mode, msg_source) = if repo_state != RepoState::Clean
			&& reword.is_some()
		{
			bail!("cannot reword while repo is not in a clean state");
		} else if let Some(reword_id) = reword {
			self.input.set_text(
				sync::get_commit_details(
					&self.repo.borrow(),
					reword_id,
				)?
				.message
				.unwrap_or_default()
				.combine(),
			);
			self.input.set_title(strings::commit_reword_title());
			(Mode::Reword(reword_id), PrepareCommitMsgSource::Message)
		} else {
			match repo_state {
				RepoState::Merge => {
					let ids =
						sync::mergehead_ids(&self.repo.borrow())?;
					self.input
						.set_title(strings::commit_title_merge());
					self.input.set_text(sync::merge_msg(
						&self.repo.borrow(),
					)?);
					(Mode::Merge(ids), PrepareCommitMsgSource::Merge)
				}
				RepoState::Revert => {
					self.input
						.set_title(strings::commit_title_revert());
					self.input.set_text(sync::merge_msg(
						&self.repo.borrow(),
					)?);
					(Mode::Revert, PrepareCommitMsgSource::Message)
				}

				_ => {
					self.commit_template = get_config_string(
						&self.repo.borrow(),
						"commit.template",
					)
					.map_err(|e| {
						log::error!("load git-config failed: {}", e);
						e
					})
					.ok()
					.flatten()
					.and_then(|path| {
						shellexpand::full(path.as_str())
							.ok()
							.and_then(|path| {
								PathBuf::from_str(path.as_ref()).ok()
							})
					})
					.and_then(|path| {
						read_to_string(&path)
							.map_err(|e| {
								log::error!("read commit.template failed: {e} (path: '{:?}')",path);
								e
							})
							.ok()
					});

					let msg_source = if self.is_empty() {
						if let Some(s) = &self.commit_template {
							self.input.set_text(s.clone());
							PrepareCommitMsgSource::Template
						} else {
							PrepareCommitMsgSource::Message
						}
					} else {
						PrepareCommitMsgSource::Message
					};
					self.input.set_title(strings::commit_title());

					(Mode::Normal, msg_source)
				}
			}
		};

		self.mode = mode;

		let mut msg = self.input.get_text().to_string();
		if let HookResult::NotOk(e) = sync::hooks_prepare_commit_msg(
			&self.repo.borrow(),
			msg_source,
			&mut msg,
		)? {
			log::error!("prepare-commit-msg hook rejection: {e}",);
		}
		self.input.set_text(msg);

		self.commit_msg_history_idx = 0;
		self.input.show()?;

		Ok(())
	}

	fn add_sign_off(&self, msg: &str) -> Result<String> {
		const CONFIG_KEY_USER_NAME: &str = "user.name";
		const CONFIG_KEY_USER_MAIL: &str = "user.email";

		let user = get_config_string(
			&self.repo.borrow(),
			CONFIG_KEY_USER_NAME,
		)?;

		let mail = get_config_string(
			&self.repo.borrow(),
			CONFIG_KEY_USER_MAIL,
		)?;

		let mut msg = msg.to_owned();
		if let (Some(user), Some(mail)) = (user, mail) {
			msg.push_str(&format!(
				"\n\nSigned-off-by {user} <{mail}>"
			));
		}

		Ok(msg)
	}
}

impl DrawableComponent for CommitComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		rect: Rect,
	) -> Result<()> {
		if self.is_visible() {
			self.input.draw(f, rect)?;
			self.draw_branch_name(f);
			self.draw_warnings(f);
		}

		Ok(())
	}
}

impl Component for CommitComponent {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		self.input.commands(out, force_all);

		if self.is_visible() || force_all {
			out.push(CommandInfo::new(
				strings::commands::commit_enter(&self.key_config),
				self.can_commit(),
				true,
			));

			out.push(CommandInfo::new(
				strings::commands::toggle_verify(
					&self.key_config,
					self.verify,
				),
				self.can_commit(),
				true,
			));

			out.push(CommandInfo::new(
				strings::commands::commit_amend(&self.key_config),
				self.can_amend(),
				true,
			));

			out.push(CommandInfo::new(
				strings::commands::commit_signoff(&self.key_config),
				true,
				true,
			));

			out.push(CommandInfo::new(
				strings::commands::commit_open_editor(
					&self.key_config,
				),
				true,
				true,
			));

			out.push(CommandInfo::new(
				strings::commands::commit_next_msg_from_history(
					&self.key_config,
				),
				self.options.borrow().has_commit_msg_history(),
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
				if key_match(e, self.key_config.keys.enter)
					&& self.can_commit()
				{
					try_or_popup!(
						self,
						"commit error:",
						self.commit()
					);
				} else if key_match(
					e,
					self.key_config.keys.toggle_verify,
				) && self.can_commit()
				{
					self.toggle_verify();
				} else if key_match(
					e,
					self.key_config.keys.commit_amend,
				) && self.can_amend()
				{
					self.amend()?;
				} else if key_match(
					e,
					self.key_config.keys.open_commit_editor,
				) {
					self.queue.push(
						InternalEvent::OpenExternalEditor(None),
					);
					self.hide();
				} else if key_match(
					e,
					self.key_config.keys.commit_history_next,
				) {
					if let Some(msg) = self
						.options
						.borrow()
						.commit_msg(self.commit_msg_history_idx)
					{
						self.input.set_text(msg);
						self.commit_msg_history_idx += 1;
					}
				} else if key_match(
					e,
					self.key_config.keys.toggle_signoff,
				) {
					self.signoff_commit();
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
		self.open(None)?;
		Ok(())
	}
}
