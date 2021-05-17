use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent, EventState,
};
use crate::{
    keys::SharedKeyConfig,
    queue::Queue,
    strings,
    ui::{self, style::SharedTheme},
};
use anyhow::Result;
use asyncgit::{
    sync::{self, CommitId, TreeFile},
    AsyncNotification, CWD,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use tui::{
    backend::Backend, layout::Rect, text::Span, widgets::Clear, Frame,
};

pub struct RevisionFilesComponent {
    title: String,
    theme: SharedTheme,
    // queue: Queue,
    files: Vec<TreeFile>,
    revision: Option<CommitId>,
    visible: bool,
    key_config: SharedKeyConfig,
    current_height: std::cell::Cell<usize>,
}

impl RevisionFilesComponent {
    ///
    pub fn new(
        _queue: &Queue,
        _sender: &Sender<AsyncNotification>,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            title: String::new(),
            theme,
            files: Vec::new(),
            revision: None,
            // queue: queue.clone(),
            visible: false,
            key_config,
            current_height: std::cell::Cell::new(0),
        }
    }

    ///
    pub fn open(&mut self, commit: CommitId) -> Result<()> {
        self.files = sync::tree_files(CWD, commit)?;
        self.revision = Some(commit);
        self.title = format!(
            "File Tree at {}",
            self.revision
                .map(|c| c.get_short_string())
                .unwrap_or_default()
        );
        self.show()?;

        Ok(())
    }
}

impl DrawableComponent for RevisionFilesComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        area: Rect,
    ) -> Result<()> {
        if self.is_visible() {
            let items = self.files.iter().map(|f| {
                Span::styled(
                    f.path.to_string_lossy(),
                    self.theme.text(true, false),
                )
            });

            f.render_widget(Clear, area);
            ui::draw_list(
                f,
                area,
                &self.title,
                items,
                true,
                &self.theme,
            );

            self.current_height.set(area.height.into());
        }

        Ok(())
    }
}

impl Component for RevisionFilesComponent {
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
        }

        visibility_blocking(self)
    }

    fn event(
        &mut self,
        event: crossterm::event::Event,
    ) -> Result<EventState> {
        if self.is_visible() {
            if let Event::Key(key) = event {
                if key == self.key_config.exit_popup {
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
        self.visible = false
    }

    fn show(&mut self) -> Result<()> {
        self.visible = true;

        Ok(())
    }
}
