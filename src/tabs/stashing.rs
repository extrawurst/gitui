use crate::{
    components::{
        CommandBlocking, CommandInfo, Component, DrawableComponent,
        FileTreeComponent,
    },
    keys,
    queue::{InternalEvent, NeedsUpdate, Queue},
    strings,
    ui::style::Theme,
};
use asyncgit::{
    sync::{self, status::StatusType},
    AsyncNotification, AsyncStatus2, StatusParams, CWD,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use std::borrow::Cow;
use strings::commands;
use tui::{
    layout::{Alignment, Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Text},
};

struct Options {
    stash_untracked: bool,
    stash_indexed: bool,
}

pub struct Stashing {
    visible: bool,
    options: Options,
    index: FileTreeComponent,
    theme: Theme,
    git_status: AsyncStatus2,
    queue: Queue,
}

impl Stashing {
    ///
    pub fn new(
        sender: &Sender<AsyncNotification>,
        queue: &Queue,
        theme: &Theme,
    ) -> Self {
        Self {
            visible: false,
            options: Options {
                stash_indexed: true,
                stash_untracked: true,
            },
            index: FileTreeComponent::new(
                strings::STASHING_FILES_TITLE,
                true,
                queue.clone(),
                theme,
            ),
            theme: *theme,
            git_status: AsyncStatus2::new(sender.clone()),
            queue: queue.clone(),
        }
    }

    ///
    pub fn update(&mut self) {
        let status_type = if self.options.stash_indexed {
            StatusType::Both
        } else {
            StatusType::WorkingDir
        };

        self.git_status
            .fetch(StatusParams::new(
                status_type,
                self.options.stash_untracked,
            ))
            .unwrap();
    }

    ///
    pub fn anything_pending(&self) -> bool {
        self.git_status.is_pending()
    }

    ///
    pub fn update_git(&mut self, ev: AsyncNotification) {
        if self.visible {
            if let AsyncNotification::Status = ev {
                let status = self.git_status.last().unwrap();
                self.index.update(&status.items);
            }
        }
    }

    fn get_option_text(&self) -> Vec<Text> {
        let bracket_open = Text::Raw(Cow::from("["));
        let bracket_close = Text::Raw(Cow::from("]"));
        let option_on =
            Text::Styled(Cow::from("x"), self.theme.option(true));

        let option_off =
            Text::Styled(Cow::from("_"), self.theme.option(false));

        vec![
            bracket_open.clone(),
            if self.options.stash_untracked {
                option_on.clone()
            } else {
                option_off.clone()
            },
            bracket_close.clone(),
            Text::Raw(Cow::from(" stash untracked\n")),
            bracket_open,
            if self.options.stash_indexed {
                option_on.clone()
            } else {
                option_off.clone()
            },
            bracket_close,
            Text::Raw(Cow::from(" stash staged")),
        ]
    }
}

impl DrawableComponent for Stashing {
    fn draw<B: tui::backend::Backend>(
        &mut self,
        f: &mut tui::Frame<B>,
        rect: tui::layout::Rect,
    ) {
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

        self.index.draw(f, chunks[0]);

        f.render_widget(
            Paragraph::new(self.get_option_text().iter())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(strings::STASHING_OPTIONS_TITLE),
                )
                .alignment(Alignment::Left),
            right_chunks[0],
        );
    }
}

impl Component for Stashing {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        self.index.commands(out, force_all);

        out.push(CommandInfo::new(
            commands::STASHING_SAVE,
            self.visible && !self.index.is_empty(),
            self.visible || force_all,
        ));
        out.push(CommandInfo::new(
            commands::STASHING_TOGGLE_INDEXED,
            self.visible,
            self.visible || force_all,
        ));
        out.push(CommandInfo::new(
            commands::STASHING_TOGGLE_UNTRACKED,
            self.visible,
            self.visible || force_all,
        ));

        if self.visible {
            CommandBlocking::Blocking
        } else {
            CommandBlocking::PassingOn
        }
    }

    fn event(&mut self, ev: crossterm::event::Event) -> bool {
        if self.visible {
            let conusmed = self.index.event(ev);

            if conusmed {
                return true;
            }

            if let Event::Key(k) = ev {
                return match k {
                    keys::STASHING_SAVE if !self.index.is_empty() => {
                        if sync::stash_save(
                            CWD,
                            None,
                            self.options.stash_untracked,
                            !self.options.stash_indexed,
                        )
                        .is_ok()
                        {
                            self.queue.borrow_mut().push_back(
                                InternalEvent::Update(
                                    NeedsUpdate::ALL,
                                ),
                            );
                        }
                        true
                    }
                    keys::STASHING_TOGGLE_INDEX => {
                        self.options.stash_indexed =
                            !self.options.stash_indexed;
                        self.update();
                        true
                    }
                    keys::STASHING_TOGGLE_UNTRACKED => {
                        self.options.stash_untracked =
                            !self.options.stash_untracked;
                        self.update();
                        true
                    }
                    _ => false,
                };
            }
        }

        false
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn show(&mut self) {
        self.update();
        self.visible = true;
    }
}
