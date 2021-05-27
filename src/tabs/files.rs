#![allow(
    dead_code,
    clippy::missing_const_for_fn,
    clippy::unused_self
)]

use crate::{
    components::{
        visibility_blocking, CommandBlocking, CommandInfo, Component,
        DrawableComponent, EventState,
    },
    keys::SharedKeyConfig,
    queue::Queue,
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::AsyncNotification;
use crossbeam_channel::Sender;

pub struct FilesTab {
    visible: bool,
    theme: SharedTheme,
    queue: Queue,
    key_config: SharedKeyConfig,
}

impl FilesTab {
    ///
    pub fn new(
        _sender: &Sender<AsyncNotification>,
        queue: &Queue,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            visible: false,
            theme,
            queue: queue.clone(),
            key_config,
        }
    }

    ///
    pub fn update(&mut self) -> Result<()> {
        if self.is_visible() {
            //check if head changed
        }

        Ok(())
    }

    ///
    pub fn anything_pending(&self) -> bool {
        //TODO
        false
    }

    ///
    pub fn update_git(
        &mut self,
        _ev: AsyncNotification,
    ) -> Result<()> {
        if self.is_visible() {
            //forward
        }

        Ok(())
    }
}

impl DrawableComponent for FilesTab {
    fn draw<B: tui::backend::Backend>(
        &self,
        _f: &mut tui::Frame<B>,
        _rect: tui::layout::Rect,
    ) -> Result<()> {
        Ok(())
    }
}

impl Component for FilesTab {
    fn commands(
        &self,
        _out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.visible || force_all {
            //
        }

        visibility_blocking(self)
    }

    fn event(
        &mut self,
        _ev: crossterm::event::Event,
    ) -> Result<EventState> {
        if self.visible {
            //
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
