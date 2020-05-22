use crate::{
    accessors,
    components::{
        command_pump, event_pump, visibility_blocking,
        CommandBlocking, CommandInfo, Component, DrawableComponent,
        FileTreeComponent,
    },
    keys,
    queue::{InternalEvent, Queue},
    strings,
    ui::style::Theme,
};
use asyncgit::{
    sync::status::StatusType, AsyncNotification, AsyncStatus,
    StatusParams,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use std::borrow::Cow;
use strings::commands;
use tui::{
    layout::{Alignment, Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Text},
};

#[derive(Default, Clone, Copy)]
pub struct StashingOptions {
    pub stash_untracked: bool,
    pub keep_index: bool,
}

pub struct Stashing {
    index: FileTreeComponent,
    visible: bool,
    options: StashingOptions,
    theme: Theme,
    git_status: AsyncStatus,
    queue: Queue,
}

impl Stashing {
    accessors!(self, [index]);

    ///
    pub fn new(
        sender: &Sender<AsyncNotification>,
        queue: &Queue,
        theme: &Theme,
    ) -> Self {
        Self {
            index: FileTreeComponent::new(
                strings::STASHING_FILES_TITLE,
                true,
                queue.clone(),
                theme,
            ),
            visible: false,
            options: StashingOptions {
                keep_index: false,
                stash_untracked: true,
            },
            theme: *theme,
            git_status: AsyncStatus::new(sender.clone()),
            queue: queue.clone(),
        }
    }

    ///
    pub fn update(&mut self) {
        if self.visible {
            self.git_status
                .fetch(StatusParams::new(
                    StatusType::Both,
                    self.options.stash_untracked,
                ))
                .unwrap();
        }
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
            if self.options.keep_index {
                option_on.clone()
            } else {
                option_off.clone()
            },
            bracket_close,
            Text::Raw(Cow::from(" keep index")),
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

        self.index.draw(f, chunks[0]);
    }
}

impl Component for Stashing {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        command_pump(out, force_all, self.components().as_slice());

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

        visibility_blocking(self)
    }

    fn event(&mut self, ev: crossterm::event::Event) -> bool {
        if self.visible {
            if event_pump(ev, self.components_mut().as_mut_slice()) {
                return true;
            }

            if let Event::Key(k) = ev {
                return match k {
                    keys::STASHING_SAVE if !self.index.is_empty() => {
                        self.queue.borrow_mut().push_back(
                            InternalEvent::PopupStashing(
                                self.options,
                            ),
                        );

                        true
                    }
                    keys::STASHING_TOGGLE_INDEX => {
                        self.options.keep_index =
                            !self.options.keep_index;
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
        self.visible = true;
        self.update();
    }
}
