use super::{
	command_pump, event_pump, visibility_blocking, CommandBlocking,
	CommandInfo, CommitDetailsComponent, Component, DiffComponent,
	DrawableComponent, EventState,
};
use crate::{
	accessors,
	keys::{key_match, SharedKeyConfig},
	queue::{InternalEvent, Queue, StackablePopupOpen},
	strings,
	ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
	sync::{diff::DiffOptions, CommitId, CommitTags, RepoPathRef},
	AsyncDiff, AsyncGitNotification, DiffParams, DiffType,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use ratatui::{
	backend::Backend,
	layout::{Constraint, Direction, Layout, Rect},
	widgets::Clear,
	Frame,
};

#[derive(Clone, Debug)]
pub struct InspectCommitOpen {
	pub commit_id: CommitId,
	/// in case we wanna compare
	pub compare_id: Option<CommitId>,
	pub tags: Option<CommitTags>,
}

impl InspectCommitOpen {
	pub const fn new(commit_id: CommitId) -> Self {
		Self {
			commit_id,
			compare_id: None,
			tags: None,
		}
	}

	pub const fn new_with_tags(
		commit_id: CommitId,
		tags: Option<CommitTags>,
	) -> Self {
		Self {
			commit_id,
			compare_id: None,
			tags,
		}
	}
}

pub struct InspectCommitComponent {
	queue: Queue,
	open_request: Option<InspectCommitOpen>,
	diff: DiffComponent,
	details: CommitDetailsComponent,
	git_diff: AsyncDiff,
	visible: bool,
	key_config: SharedKeyConfig,
}

impl DrawableComponent for InspectCommitComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		rect: Rect,
	) -> Result<()> {
		if self.is_visible() {
			let percentages = if self.diff.focused() {
				(0, 100)
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

impl Component for InspectCommitComponent {
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
				strings::commands::close_popup(&self.key_config),
				true,
				self.diff.focused() || force_all,
			));

			out.push(CommandInfo::new(
				strings::commands::inspect_file_tree(
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
			if event_pump(ev, self.components_mut().as_mut_slice())?
				.is_consumed()
			{
				if !self.details.is_visible() {
					self.hide_stacked(true);
				}

				return Ok(EventState::Consumed);
			}

			if let Event::Key(e) = ev {
				if key_match(e, self.key_config.keys.exit_popup) {
					if self.diff.focused() {
						self.details.focus(true);
						self.diff.focus(false);
					} else {
						self.hide_stacked(false);
					}
				} else if key_match(
					e,
					self.key_config.keys.move_right,
				) && self.can_focus_diff()
				{
					self.details.focus(false);
					self.diff.focus(true);
				} else if key_match(e, self.key_config.keys.move_left)
				{
					self.hide_stacked(false);
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

impl InspectCommitComponent {
	accessors!(self, [diff, details]);

	///
	pub fn new(
		repo: &RepoPathRef,
		queue: &Queue,
		sender: &Sender<AsyncGitNotification>,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			queue: queue.clone(),
			details: CommitDetailsComponent::new(
				repo,
				queue,
				sender,
				theme.clone(),
				key_config.clone(),
			),
			diff: DiffComponent::new(
				repo.clone(),
				queue.clone(),
				theme,
				key_config.clone(),
				true,
			),
			open_request: None,
			git_diff: AsyncDiff::new(repo.borrow().clone(), sender),
			visible: false,
			key_config,
		}
	}

	///
	pub fn open(&mut self, open: InspectCommitOpen) -> Result<()> {
		self.open_request = Some(open);
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
			if ev == AsyncGitNotification::CommitFiles {
				self.update()?;
			} else if ev == AsyncGitNotification::Diff {
				self.update_diff()?;
			}
		}

		Ok(())
	}

	/// called when any tree component changed selection
	pub fn update_diff(&mut self) -> Result<()> {
		if self.is_visible() {
			if let Some(request) = &self.open_request {
				if let Some(f) = self.details.files().selection_file()
				{
					let diff_params = DiffParams {
						path: f.path.clone(),
						diff_type: DiffType::Commit(
							request.commit_id,
						),
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
		if let Some(request) = &self.open_request {
			self.details.set_commits(
				Some(request.commit_id.into()),
				&request.tags,
			)?;
			self.update_diff()?;
		}

		Ok(())
	}

	fn can_focus_diff(&self) -> bool {
		self.details.files().selection_file().is_some()
	}

	fn hide_stacked(&mut self, stack: bool) {
		self.hide();

		if stack {
			if let Some(open_request) = self.open_request.take() {
				self.queue.push(InternalEvent::PopupStackPush(
					StackablePopupOpen::InspectCommit(open_request),
				));
			}
		} else {
			self.queue.push(InternalEvent::PopupStackPop);
		}
	}
}
