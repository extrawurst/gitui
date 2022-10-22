use crate::{
	components::{
		visibility_blocking, CommandBlocking, CommandInfo, Component,
		DrawableComponent, EventState, WorkTreesComponent,
	}, ui::style::SharedTheme, keys::SharedKeyConfig,
};
use anyhow::Result;
use asyncgit::sync::{RepoPathRef, worktrees};


pub struct WorkTreesTab {
	repo: RepoPathRef,
	visible: bool,
    worktrees: WorkTreesComponent,
    key_config: SharedKeyConfig,
}

impl WorkTreesTab {
	///
	pub fn new(
		repo: RepoPathRef,
	    theme: SharedTheme,
        key_config: SharedKeyConfig,
	) -> Self {
		Self {
			visible: false,
            worktrees: WorkTreesComponent::new(
                "Hello Worktrees",
                repo.clone(),
                theme,
                key_config.clone(),
            ),
			repo,
            key_config,
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
}

impl DrawableComponent for WorkTreesTab {
	fn draw<B: tui::backend::Backend>(
		&self,
		f: &mut tui::Frame<B>,
		rect: tui::layout::Rect,
	) -> Result<()> {
		if self.is_visible() {
            // TODO: Do stuff
			//self.files.draw(f, rect)?;
            self.worktrees.draw(f, rect)?;
            log::trace!("trying to draw worktrees");
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
		visibility_blocking(self)
	}

	fn event(
		&mut self,
		ev: &crossterm::event::Event,
	) -> Result<EventState> {
        if !self.visible {
		    return Ok(EventState::NotConsumed);
        }
        log::trace!("TODO: delete me {:?}", self.key_config.keys.tab_status);
        log::trace!("TODO: delete me {:?}", ev);
        let event_used = self.worktrees.event(ev)?;

        if event_used.is_consumed() {
            self.update()?;
            return Ok(EventState::Consumed);
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