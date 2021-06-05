#![allow(
    dead_code,
    clippy::missing_const_for_fn,
    clippy::unused_self
)]

use crate::{
    components::{
        visibility_blocking, CommandBlocking, CommandInfo, Component,
        DrawableComponent, EventState, RevisionFilesComponent,
    },
    keys::SharedKeyConfig,
    queue::Queue,
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{sync, AsyncNotification, CWD};
use crossbeam_channel::Sender;

pub struct FilesTab {
    visible: bool,
    theme: SharedTheme,
    queue: Queue,
    key_config: SharedKeyConfig,
    files: RevisionFilesComponent,
}

impl FilesTab {
    ///
    pub fn new(
        sender: &Sender<AsyncNotification>,
        queue: &Queue,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            visible: false,
            queue: queue.clone(),
            files: RevisionFilesComponent::new(
                queue,
                sender,
                theme.clone(),
                key_config.clone(),
            ),
            theme,
            key_config,
        }
    }

    ///
    pub fn update(&mut self) -> Result<()> {
        if self.is_visible() {
            self.files.set_commit(sync::get_head(CWD)?)?;
        }

        Ok(())
    }

    ///
    pub fn anything_pending(&self) -> bool {
        self.files.any_work_pending()
    }

    ///
    pub fn update_git(&mut self, ev: AsyncNotification) {
        if self.is_visible() {
            self.files.update(ev);
        }
    }
}

impl DrawableComponent for FilesTab {
    fn draw<B: tui::backend::Backend>(
        &self,
        f: &mut tui::Frame<B>,
        rect: tui::layout::Rect,
    ) -> Result<()> {
        if self.is_visible() {
            self.files.draw(f, rect)?;
        }
        Ok(())
    }
}

impl Component for FilesTab {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.visible || force_all {
            return self.files.commands(out, force_all);
        }

        visibility_blocking(self)
    }

    fn event(
        &mut self,
        ev: crossterm::event::Event,
    ) -> Result<EventState> {
        if self.visible {
            return self.files.event(ev);
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
