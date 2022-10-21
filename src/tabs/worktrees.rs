use crate::{
	components::{
		visibility_blocking, CommandBlocking, CommandInfo, Component,
		DrawableComponent, EventState,
	},
};
use anyhow::Result;
use asyncgit::sync::RepoPathRef;


pub struct WorkTreesTab {
	repo: RepoPathRef,
	visible: bool,
}

impl WorkTreesTab {
	///
	pub fn new(
		repo: RepoPathRef,
	) -> Self {
		Self {
			visible: false,
			repo,
		}
	}
	
    pub fn update(&mut self) -> Result<()> {
        log::trace!("repo: {:?}", self.repo);
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
