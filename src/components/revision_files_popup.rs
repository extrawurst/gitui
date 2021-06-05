use super::{
    revision_files::RevisionFilesComponent, visibility_blocking,
    CommandBlocking, CommandInfo, Component, DrawableComponent,
    EventState,
};
use crate::{
    keys::SharedKeyConfig,
    queue::Queue,
    strings::{self},
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{sync::CommitId, AsyncNotification};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};

pub struct RevisionFilesPopup {
    visible: bool,
    key_config: SharedKeyConfig,
    files: RevisionFilesComponent,
}

impl RevisionFilesPopup {
    ///
    pub fn new(
        queue: &Queue,
        sender: &Sender<AsyncNotification>,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            files: RevisionFilesComponent::new(
                queue,
                sender,
                theme,
                key_config.clone(),
            ),
            visible: false,
            key_config,
        }
    }

    ///
    pub fn open(&mut self, commit: CommitId) -> Result<()> {
        self.files.set_commit(commit)?;
        self.show()?;

        Ok(())
    }

    ///
    pub fn update(&mut self, ev: AsyncNotification) {
        self.files.update(ev);
    }

    ///
    pub fn any_work_pending(&self) -> bool {
        self.files.any_work_pending()
    }
}

impl DrawableComponent for RevisionFilesPopup {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        area: Rect,
    ) -> Result<()> {
        if self.is_visible() {
            f.render_widget(Clear, area);
            // f.render_widget(
            //     Block::default()
            //         .borders(Borders::TOP)
            //         .title(Span::styled(
            //             format!(" {}", self.title),
            //             self.theme.title(true),
            //         ))
            //         .border_style(self.theme.block(true)),
            //     area,
            // );

            self.files.draw(f, area)?;
        }

        Ok(())
    }
}

impl Component for RevisionFilesPopup {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.is_visible() || force_all {
            out.push(
                CommandInfo::new(
                    strings::commands::close_popup(&self.key_config),
                    true,
                    true,
                )
                .order(1),
            );

            self.files.commands(out, force_all);
        }

        visibility_blocking(self)
    }

    fn event(
        &mut self,
        event: crossterm::event::Event,
    ) -> Result<EventState> {
        if self.is_visible() {
            if let Event::Key(key) = &event {
                if *key == self.key_config.exit_popup {
                    self.hide();
                }
            }

            return self.files.event(event);
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

        Ok(())
    }
}
