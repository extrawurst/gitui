use crate::{
	components::{
		visibility_blocking, CommandBlocking, CommandInfo,
		CommitDetailsComponent, CommitList, Component,
		DrawableComponent, EventState, FileTreeOpen,
		InspectCommitOpen,
	},
	keys::{key_match, SharedKeyConfig},
	queue::{InternalEvent, Queue, StackablePopupOpen},
	strings, try_or_popup,
	ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
	asyncjob::AsyncSingleJob,
	sync::{self, CommitId, RepoPathRef},
	AsyncBranchesJob, AsyncGitNotification, AsyncLog, AsyncTags,
	CommitFilesParams, FetchStatus,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use ratatui::{
	backend::Backend,
	layout::{Constraint, Direction, Layout, Rect},
	Frame,
};
use std::time::Duration;
use sync::CommitTags;

const SLICE_SIZE: usize = 1200;

///
pub struct Revlog {
	repo: RepoPathRef,
	commit_details: CommitDetailsComponent,
	list: CommitList,
	git_log: AsyncLog,
	git_tags: AsyncTags,
	git_local_branches: AsyncSingleJob<AsyncBranchesJob>,
	git_remote_branches: AsyncSingleJob<AsyncBranchesJob>,
	queue: Queue,
	visible: bool,
	key_config: SharedKeyConfig,
}

impl Revlog {
	///
	pub fn new(
		repo: &RepoPathRef,
		queue: &Queue,
		sender: &Sender<AsyncGitNotification>,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			repo: repo.clone(),
			queue: queue.clone(),
			commit_details: CommitDetailsComponent::new(
				repo,
				queue,
				sender,
				theme.clone(),
				key_config.clone(),
			),
			list: CommitList::new(
				repo.clone(),
				&strings::log_title(&key_config),
				theme,
				queue.clone(),
				key_config.clone(),
			),
			git_log: AsyncLog::new(
				repo.borrow().clone(),
				sender,
				None,
			),
			git_tags: AsyncTags::new(repo.borrow().clone(), sender),
			git_local_branches: AsyncSingleJob::new(sender.clone()),
			git_remote_branches: AsyncSingleJob::new(sender.clone()),
			visible: false,
			key_config,
		}
	}

	///
	pub fn any_work_pending(&self) -> bool {
		self.git_log.is_pending()
			|| self.git_tags.is_pending()
			|| self.git_local_branches.is_pending()
			|| self.git_remote_branches.is_pending()
			|| self.commit_details.any_work_pending()
	}

	///
	pub fn update(&mut self) -> Result<()> {
		if self.is_visible() {
			let log_changed =
				self.git_log.fetch()? == FetchStatus::Started;

			self.list.set_count_total(self.git_log.count()?);

			let selection = self.list.selection();
			let selection_max = self.list.selection_max();
			if self.list.items().needs_data(selection, selection_max)
				|| log_changed
			{
				self.fetch_commits()?;
			}

			self.git_tags.request(Duration::from_secs(3), false)?;

			if self.commit_details.is_visible() {
				let commit = self.selected_commit();
				let tags = self.selected_commit_tags(&commit);

				self.commit_details.set_commits(
					commit.map(CommitFilesParams::from),
					&tags,
				)?;
			}
		}

		Ok(())
	}

	///
	pub fn update_git(
		&mut self,
		ev: AsyncGitNotification,
	) -> Result<()> {
		if self.visible {
			match ev {
				AsyncGitNotification::CommitFiles
				| AsyncGitNotification::Log => self.update()?,
				AsyncGitNotification::Tags => {
					if let Some(tags) = self.git_tags.last()? {
						self.list.set_tags(tags);
						self.update()?;
					}
				}
				AsyncGitNotification::Branches => {
					if let Some(local_branches) =
						self.git_local_branches.take_last()
					{
						if let Some(Ok(local_branches)) =
							local_branches.result()
						{
							self.list
								.set_local_branches(local_branches);
							self.update()?;
						}
					}

					if let Some(remote_branches) =
						self.git_remote_branches.take_last()
					{
						if let Some(Ok(remote_branches)) =
							remote_branches.result()
						{
							self.list
								.set_remote_branches(remote_branches);
							self.update()?;
						}
					}
				}
				_ => (),
			}
		}

		Ok(())
	}

	fn fetch_commits(&mut self) -> Result<()> {
		let want_min =
			self.list.selection().saturating_sub(SLICE_SIZE / 2);

		let commits = sync::get_commits_info(
			&self.repo.borrow(),
			&self.git_log.get_slice(want_min, SLICE_SIZE)?,
			self.list
				.current_size()
				.map_or(100u16, |size| size.0)
				.into(),
		);

		if let Ok(commits) = commits {
			self.list.items().set_items(want_min, commits);
		}

		Ok(())
	}

	fn selected_commit(&self) -> Option<CommitId> {
		self.list.selected_entry().map(|e| e.id)
	}

	fn selected_commit_tags(
		&self,
		commit: &Option<CommitId>,
	) -> Option<CommitTags> {
		let tags = self.list.tags();

		commit.and_then(|commit| {
			tags.and_then(|tags| tags.get(&commit).cloned())
		})
	}

	pub fn select_commit(&mut self, id: CommitId) -> Result<()> {
		let position = self.git_log.position(id)?;

		if let Some(position) = position {
			self.list.select_entry(position);

			Ok(())
		} else {
			anyhow::bail!("Could not select commit in revlog. It might not be loaded yet or it might be on a different branch.");
		}
	}

	fn revert_commit(&self) -> Result<()> {
		if let Some(c) = self.selected_commit() {
			sync::revert_commit(&self.repo.borrow(), c)?;
			self.queue.push(InternalEvent::TabSwitchStatus);
		}

		Ok(())
	}

	fn inspect_commit(&self) {
		if let Some(commit_id) = self.selected_commit() {
			let tags = self.selected_commit_tags(&Some(commit_id));
			self.queue.push(InternalEvent::OpenPopup(
				StackablePopupOpen::InspectCommit(
					InspectCommitOpen::new_with_tags(commit_id, tags),
				),
			));
		}
	}
}

impl DrawableComponent for Revlog {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		area: Rect,
	) -> Result<()> {
		let chunks = Layout::default()
			.direction(Direction::Horizontal)
			.constraints(
				[
					Constraint::Percentage(60),
					Constraint::Percentage(40),
				]
				.as_ref(),
			)
			.split(area);

		if self.commit_details.is_visible() {
			self.list.draw(f, chunks[0])?;
			self.commit_details.draw(f, chunks[1])?;
		} else {
			self.list.draw(f, area)?;
		}

		Ok(())
	}
}

impl Component for Revlog {
	//TODO: cleanup
	#[allow(clippy::too_many_lines)]
	fn event(&mut self, ev: &Event) -> Result<EventState> {
		if self.visible {
			let event_used = self.list.event(ev)?;

			if event_used.is_consumed() {
				self.update()?;
				return Ok(EventState::Consumed);
			} else if let Event::Key(k) = ev {
				if key_match(k, self.key_config.keys.enter) {
					self.commit_details.toggle_visible()?;
					self.update()?;
					return Ok(EventState::Consumed);
				} else if key_match(k, self.key_config.keys.copy) {
					try_or_popup!(
						self,
						strings::POPUP_FAIL_COPY,
						self.list.copy_commit_hash()
					);
					return Ok(EventState::Consumed);
				} else if key_match(k, self.key_config.keys.push) {
					self.queue.push(InternalEvent::PushTags);
					return Ok(EventState::Consumed);
				} else if key_match(
					k,
					self.key_config.keys.log_tag_commit,
				) {
					return self.selected_commit().map_or(
						Ok(EventState::NotConsumed),
						|id| {
							self.queue
								.push(InternalEvent::TagCommit(id));
							Ok(EventState::Consumed)
						},
					);
				} else if key_match(
					k,
					self.key_config.keys.move_right,
				) && self.commit_details.is_visible()
				{
					self.inspect_commit();
					return Ok(EventState::Consumed);
				} else if key_match(
					k,
					self.key_config.keys.select_branch,
				) {
					self.queue.push(InternalEvent::SelectBranch);
					return Ok(EventState::Consumed);
				} else if key_match(
					k,
					self.key_config.keys.status_reset_item,
				) {
					try_or_popup!(
						self,
						"revert error:",
						self.revert_commit()
					);

					return Ok(EventState::Consumed);
				} else if key_match(
					k,
					self.key_config.keys.open_file_tree,
				) {
					return self.selected_commit().map_or(
						Ok(EventState::NotConsumed),
						|id| {
							self.queue.push(
								InternalEvent::OpenPopup(
									StackablePopupOpen::FileTree(
										FileTreeOpen::new(id),
									),
								),
							);
							Ok(EventState::Consumed)
						},
					);
				} else if key_match(k, self.key_config.keys.tags) {
					self.queue.push(InternalEvent::Tags);
					return Ok(EventState::Consumed);
				} else if key_match(
					k,
					self.key_config.keys.log_reset_comit,
				) {
					return self.selected_commit().map_or(
						Ok(EventState::NotConsumed),
						|id| {
							self.queue.push(
								InternalEvent::OpenResetPopup(id),
							);
							Ok(EventState::Consumed)
						},
					);
				} else if key_match(
					k,
					self.key_config.keys.log_reword_comit,
				) {
					return self.selected_commit().map_or(
						Ok(EventState::NotConsumed),
						|id| {
							self.queue.push(
								InternalEvent::RewordCommit(id),
							);
							Ok(EventState::Consumed)
						},
					);
				} else if key_match(
					k,
					self.key_config.keys.compare_commits,
				) && self.list.marked_count() > 0
				{
					if self.list.marked_count() == 1 {
						// compare against head
						self.queue.push(InternalEvent::OpenPopup(
							StackablePopupOpen::CompareCommits(
								InspectCommitOpen::new(
									self.list.marked()[0].1,
								),
							),
						));
						return Ok(EventState::Consumed);
					} else if self.list.marked_count() == 2 {
						//compare two marked commits
						let marked = self.list.marked();
						self.queue.push(InternalEvent::OpenPopup(
							StackablePopupOpen::CompareCommits(
								InspectCommitOpen {
									commit_id: marked[0].1,
									compare_id: Some(marked[1].1),
									tags: None,
								},
							),
						));
						return Ok(EventState::Consumed);
					}
				}
			}
		}

		Ok(EventState::NotConsumed)
	}

	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.visible || force_all {
			self.list.commands(out, force_all);
		}

		out.push(CommandInfo::new(
			strings::commands::log_details_toggle(&self.key_config),
			true,
			self.visible,
		));

		out.push(CommandInfo::new(
			strings::commands::commit_details_open(&self.key_config),
			true,
			(self.visible && self.commit_details.is_visible())
				|| force_all,
		));

		out.push(CommandInfo::new(
			strings::commands::open_branch_select_popup(
				&self.key_config,
			),
			true,
			self.visible || force_all,
		));

		out.push(CommandInfo::new(
			strings::commands::compare_with_head(&self.key_config),
			self.list.marked_count() == 1,
			(self.visible && self.list.marked_count() <= 1)
				|| force_all,
		));

		out.push(CommandInfo::new(
			strings::commands::compare_commits(&self.key_config),
			true,
			(self.visible && self.list.marked_count() == 2)
				|| force_all,
		));

		out.push(CommandInfo::new(
			strings::commands::copy_hash(&self.key_config),
			self.selected_commit().is_some(),
			self.visible || force_all,
		));

		out.push(CommandInfo::new(
			strings::commands::log_tag_commit(&self.key_config),
			self.selected_commit().is_some(),
			self.visible || force_all,
		));

		out.push(CommandInfo::new(
			strings::commands::log_checkout_commit(&self.key_config),
			self.selected_commit().is_some(),
			self.visible || force_all,
		));

		out.push(CommandInfo::new(
			strings::commands::open_tags_popup(&self.key_config),
			true,
			self.visible || force_all,
		));

		out.push(CommandInfo::new(
			strings::commands::push_tags(&self.key_config),
			true,
			self.visible || force_all,
		));

		out.push(CommandInfo::new(
			strings::commands::inspect_file_tree(&self.key_config),
			self.selected_commit().is_some(),
			self.visible || force_all,
		));

		out.push(CommandInfo::new(
			strings::commands::revert_commit(&self.key_config),
			self.selected_commit().is_some(),
			self.visible || force_all,
		));

		out.push(CommandInfo::new(
			strings::commands::log_reset_commit(&self.key_config),
			self.selected_commit().is_some(),
			self.visible || force_all,
		));
		out.push(CommandInfo::new(
			strings::commands::log_reword_commit(&self.key_config),
			self.selected_commit().is_some(),
			self.visible || force_all,
		));

		visibility_blocking(self)
	}

	fn is_visible(&self) -> bool {
		self.visible
	}

	fn hide(&mut self) {
		self.visible = false;
		self.git_log.set_background();
	}

	fn show(&mut self) -> Result<()> {
		self.visible = true;
		self.list.clear();

		self.git_local_branches.spawn(AsyncBranchesJob::new(
			self.repo.borrow().clone(),
			true,
		));

		self.git_remote_branches.spawn(AsyncBranchesJob::new(
			self.repo.borrow().clone(),
			false,
		));

		self.update()?;

		Ok(())
	}
}
