use crate::{
	components::{
		visibility_blocking, CommandBlocking, CommandInfo, Component,
		DrawableComponent, EventState, WorkTreesComponent,
	},
	keys::{key_match, SharedKeyConfig},
	queue::{InternalEvent, Queue},
	strings,
	ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::sync::{worktrees, RepoPathRef, WorkTree};
use crossterm::event::Event;

pub struct WorkTreesTab {
	repo: RepoPathRef,
	visible: bool,
	worktrees: WorkTreesComponent,
	key_config: SharedKeyConfig,
	queue: Queue,
}

impl WorkTreesTab {
	///
	pub fn new(
		repo: RepoPathRef,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
		queue: &Queue,
	) -> Self {
		Self {
			visible: false,
			worktrees: WorkTreesComponent::new(
				"Hello Worktrees",
				theme,
				key_config.clone(),
			),
			repo,
			key_config,
			queue: queue.clone(),
		}
	}

	pub fn update(&mut self) -> Result<()> {
		if self.is_visible() {
			if let Ok(worktrees) = worktrees(&self.repo.borrow()) {
				self.worktrees.set_worktrees(worktrees)?;
			}
		}

		Ok(())
	}

	pub fn selected_worktree(&self) -> &WorkTree {
		self.worktrees.selected_worktree().unwrap()
	}
}

impl DrawableComponent for WorkTreesTab {
	fn draw<B: tui::backend::Backend>(
		&self,
		f: &mut tui::Frame<B>,
		rect: tui::layout::Rect,
	) -> Result<()> {
		if self.is_visible() {
			self.worktrees.draw(f, rect)?;
		}
		Ok(())
	}
}

impl Component for WorkTreesTab {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.is_visible() || force_all {
			out.push(CommandInfo::new(
				strings::commands::open_worktree_create_popup(
					&self.key_config,
				),
				true,
				true,
			));
			out.push(CommandInfo::new(
				strings::commands::prune_worktree(&self.key_config),
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
		if !self.visible {
			return Ok(EventState::NotConsumed);
		}
		let event_used = self.worktrees.event(ev)?;

		if event_used.is_consumed() {
			self.update()?;
			return Ok(EventState::Consumed);
		} else if let Event::Key(e) = ev {
			if key_match(e, self.key_config.keys.select_worktree) {
				self.queue.push(InternalEvent::OpenWorktree(
					self.selected_worktree().name.clone(),
				));
				return Ok(EventState::Consumed);
			} else if key_match(
				e,
				self.key_config.keys.create_worktree,
			) {
				self.queue.push(InternalEvent::CreateWorktree);
			} else if key_match(
				e,
				self.key_config.keys.prune_worktree,
			) {
				self.queue.push(InternalEvent::PruneWorktree(
					self.selected_worktree().name.clone(),
				));
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
		self.update()?;
		Ok(())
	}
}
