use super::{
	command_pump, event_pump, visibility_blocking, CommandBlocking,
	CommandInfo, CommitDetailsComponent, Component, DiffComponent,
	DrawableComponent, EventState,
};
use crate::{
	accessors, keys::SharedKeyConfig, queue::Queue, strings,
	ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
	sync::{self, diff::DiffOptions, CommitId},
	AsyncDiff, AsyncGitNotification, CommitFilesParams, DiffParams,
	DiffType, CWD,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use tui::{
	backend::Backend,
	layout::{Constraint, Direction, Layout, Rect},
	widgets::Clear,
	Frame,
};

pub struct CompareCommitsComponent {
	commit_ids: Option<(CommitId, CommitId)>,
	diff: DiffComponent,
	details: CommitDetailsComponent,
	git_diff: AsyncDiff,
	visible: bool,
	key_config: SharedKeyConfig,
}

impl DrawableComponent for CompareCommitsComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		rect: Rect,
	) -> Result<()> {
		if self.is_visible() {
			let percentages = if self.diff.focused() {
				(30, 70)
			} else {
				(50, 50)
			};

			let chunks = Layout::default()
				.direction(Direction::Horizontal)
				.constraints(
					[
						Constraint::Percentage(percentages.0),
						Constraint::Percentage(percentages.1),
					]
					.as_ref(),
				)
				.split(rect);

			f.render_widget(Clear, rect);

			self.details.draw(f, chunks[0])?;
			self.diff.draw(f, chunks[1])?;
		}

		Ok(())
	}
}

impl Component for CompareCommitsComponent {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.is_visible() || force_all {
			command_pump(
				out,
				force_all,
				self.components().as_slice(),
			);

			out.push(
				CommandInfo::new(
					strings::commands::close_popup(&self.key_config),
					true,
					true,
				)
				.order(1),
			);

			out.push(CommandInfo::new(
				strings::commands::diff_focus_right(&self.key_config),
				self.can_focus_diff(),
				!self.diff.focused() || force_all,
			));

			out.push(CommandInfo::new(
				strings::commands::diff_focus_left(&self.key_config),
				true,
				self.diff.focused() || force_all,
			));
		}

		visibility_blocking(self)
	}

	fn event(&mut self, ev: Event) -> Result<EventState> {
		if self.is_visible() {
			if event_pump(ev, self.components_mut().as_mut_slice())?
				.is_consumed()
			{
				return Ok(EventState::Consumed);
			}

			if let Event::Key(e) = ev {
				if e == self.key_config.exit_popup {
					self.hide();
				} else if e == self.key_config.focus_right
					&& self.can_focus_diff()
				{
					self.details.focus(false);
					self.diff.focus(true);
				} else if e == self.key_config.focus_left
					&& self.diff.focused()
				{
					self.details.focus(true);
					self.diff.focus(false);
				} else if e == self.key_config.focus_left {
					self.hide();
				}

				return Ok(EventState::Consumed);
			}
		}

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
		self.details.show()?;
		self.details.focus(true);
		self.diff.focus(false);
		self.update()?;
		Ok(())
	}
}

impl CompareCommitsComponent {
	accessors!(self, [diff, details]);

	///
	pub fn new(
		queue: &Queue,
		sender: &Sender<AsyncGitNotification>,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			details: CommitDetailsComponent::new(
				queue,
				sender,
				theme.clone(),
				key_config.clone(),
			),
			diff: DiffComponent::new(
				queue.clone(),
				theme,
				key_config.clone(),
				true,
			),
			commit_ids: None,
			git_diff: AsyncDiff::new(sender),
			visible: false,
			key_config,
		}
	}

	///
	pub fn open(
		&mut self,
		id: CommitId,
		other: Option<CommitId>,
	) -> Result<()> {
		let other = if let Some(other) = other {
			other
		} else {
			sync::get_head_tuple(CWD)?.id
		};
		self.commit_ids = Some((id, other));
		self.show()?;

		Ok(())
	}

	///
	pub fn any_work_pending(&self) -> bool {
		self.git_diff.is_pending() || self.details.any_work_pending()
	}

	///
	pub fn update_git(
		&mut self,
		ev: AsyncGitNotification,
	) -> Result<()> {
		if self.is_visible() {
			if let AsyncGitNotification::CommitFiles = ev {
				self.update()?;
			} else if let AsyncGitNotification::Diff = ev {
				self.update_diff()?;
			}
		}

		Ok(())
	}

	/// called when any tree component changed selection
	pub fn update_diff(&mut self) -> Result<()> {
		if self.is_visible() {
			if let Some(ids) = self.commit_ids {
				if let Some(f) = self.details.files().selection_file()
				{
					let diff_params = DiffParams {
						path: f.path.clone(),
						diff_type: DiffType::Commits(ids),
						options: DiffOptions::default(),
					};

					if let Some((params, last)) =
						self.git_diff.last()?
					{
						if params == diff_params {
							self.diff.update(f.path, false, last);
							return Ok(());
						}
					}

					self.git_diff.request(diff_params)?;
					self.diff.clear(true);
					return Ok(());
				}
			}

			self.diff.clear(false);
		}

		Ok(())
	}

	fn update(&mut self) -> Result<()> {
		self.details.set_commits(
			self.commit_ids.map(CommitFilesParams::from),
			None,
		)?;
		self.update_diff()?;

		Ok(())
	}

	fn can_focus_diff(&self) -> bool {
		self.details.files().selection_file().is_some()
	}
}
