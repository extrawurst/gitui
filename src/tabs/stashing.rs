use crate::{
    accessors,
    components::{
        command_pump, event_pump, visibility_blocking,
        CommandBlocking, CommandInfo, Component, DrawableComponent,
        EventState, FileTreeComponent,
    },
    keys::SharedKeyConfig,
    queue::{InternalEvent, Queue},
    strings,
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
    sync::{self, status::StatusType},
    AsyncNotification, AsyncStatus, StatusParams, CWD,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use std::borrow::Cow;
use tui::{
    layout::{Alignment, Constraint, Direction, Layout},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
};

#[derive(Default, Clone, Copy, Debug)]
pub struct StashingOptions {
    pub stash_untracked: bool,
    pub keep_index: bool,
}

pub struct Stashing {
    index: FileTreeComponent,
    visible: bool,
    options: StashingOptions,
    theme: SharedTheme,
    git_status: AsyncStatus,
    queue: Queue,
    key_config: SharedKeyConfig,
}

impl Stashing {
    accessors!(self, [index]);

    ///
    pub fn new(
        sender: &Sender<AsyncNotification>,
        queue: &Queue,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            index: FileTreeComponent::new(
                &strings::stashing_files_title(&key_config),
                true,
                Some(queue.clone()),
                theme.clone(),
                key_config.clone(),
            ),
            visible: false,
            options: StashingOptions {
                keep_index: false,
                stash_untracked: true,
            },
            theme,
            git_status: AsyncStatus::new(sender.clone()),
            queue: queue.clone(),
            key_config,
        }
    }

    ///
    pub fn update(&mut self) -> Result<()> {
        if self.is_visible() {
            self.git_status
                .fetch(&StatusParams::new(StatusType::Both))?;
        }

        Ok(())
    }

    ///
    pub fn anything_pending(&self) -> bool {
        self.git_status.is_pending()
    }

    ///
    pub fn update_git(
        &mut self,
        ev: AsyncNotification,
    ) -> Result<()> {
        if self.is_visible() {
            if let AsyncNotification::Status = ev {
                let status = self.git_status.last()?;
                self.index.update(&status.items)?;
            }
        }

        Ok(())
    }

    fn get_option_text(&self) -> Vec<Spans> {
        let bracket_open = Span::raw(Cow::from("["));
        let bracket_close = Span::raw(Cow::from("]"));
        let option_on =
            Span::styled(Cow::from("x"), self.theme.option(true));

        let option_off =
            Span::styled(Cow::from("_"), self.theme.option(false));

        vec![
            Spans::from(vec![
                bracket_open.clone(),
                if self.options.stash_untracked {
                    option_on.clone()
                } else {
                    option_off.clone()
                },
                bracket_close.clone(),
                Span::raw(Cow::from(" stash untracked")),
            ]),
            Spans::from(vec![
                bracket_open,
                if self.options.keep_index {
                    option_on.clone()
                } else {
                    option_off.clone()
                },
                bracket_close,
                Span::raw(Cow::from(" keep index")),
            ]),
        ]
    }
}

impl DrawableComponent for Stashing {
    fn draw<B: tui::backend::Backend>(
        &self,
        f: &mut tui::Frame<B>,
        rect: tui::layout::Rect,
    ) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [Constraint::Min(1), Constraint::Length(22)].as_ref(),
            )
            .split(rect);

        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [Constraint::Length(4), Constraint::Min(1)].as_ref(),
            )
            .split(chunks[1]);

        f.render_widget(
            Paragraph::new(self.get_option_text())
                .block(Block::default().borders(Borders::ALL).title(
                    strings::stashing_options_title(&self.key_config),
                ))
                .alignment(Alignment::Left),
            right_chunks[0],
        );

        self.index.draw(f, chunks[0])?;

        Ok(())
    }
}

impl Component for Stashing {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.visible || force_all {
            command_pump(
                out,
                force_all,
                self.components().as_slice(),
            );

            out.push(CommandInfo::new(
                strings::commands::stashing_save(&self.key_config),
                self.visible && !self.index.is_empty(),
                self.visible || force_all,
            ));
            out.push(CommandInfo::new(
                strings::commands::stashing_toggle_indexed(
                    &self.key_config,
                ),
                self.visible,
                self.visible || force_all,
            ));
            out.push(CommandInfo::new(
                strings::commands::stashing_toggle_untracked(
                    &self.key_config,
                ),
                self.visible,
                self.visible || force_all,
            ));
        }

        visibility_blocking(self)
    }

    fn event(
        &mut self,
        ev: crossterm::event::Event,
    ) -> Result<EventState> {
        if self.visible {
            if event_pump(ev, self.components_mut().as_mut_slice())?
                .is_consumed()
            {
                return Ok(EventState::Consumed);
            }

            if let Event::Key(k) = ev {
                return if k == self.key_config.stashing_save
                    && !self.index.is_empty()
                {
                    self.queue.borrow_mut().push_back(
                        InternalEvent::PopupStashing(self.options),
                    );

                    Ok(EventState::Consumed)
                } else if k == self.key_config.stashing_toggle_index {
                    self.options.keep_index =
                        !self.options.keep_index;
                    self.update()?;
                    Ok(EventState::Consumed)
                } else if k
                    == self.key_config.stashing_toggle_untracked
                {
                    self.options.stash_untracked =
                        !self.options.stash_untracked;
                    self.update()?;
                    Ok(EventState::Consumed)
                } else {
                    Ok(EventState::NotConsumed)
                };
            };
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
        let config_untracked_files =
            sync::untracked_files_config(CWD)?;

        self.options.stash_untracked =
            !config_untracked_files.include_none();

        self.visible = true;
        self.update()?;
        Ok(())
    }
}
