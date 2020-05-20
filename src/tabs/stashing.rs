use crate::{
    components::{
        ChangesComponent, CommandBlocking, CommandInfo, Component,
        DrawableComponent,
    },
    queue::Queue,
    strings,
    ui::style::Theme,
};
use asyncgit::AsyncNotification;
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
    index: ChangesComponent,
    theme: Theme,
}

impl Stashing {
    ///
    pub fn new(
        // sender: &Sender<AsyncNotification>,
        queue: &Queue,
        theme: &Theme,
    ) -> Self {
        Self {
            visible: false,
            options: Options {
                stash_indexed: false,
                stash_untracked: true,
            },
            index: ChangesComponent::new(
                strings::STASHING_FILES_TITLE,
                true,
                true,
                queue.clone(),
                theme,
            ),
            theme: *theme,
        }
    }

    ///
    #[allow(clippy::unused_self)]
    pub fn update(&mut self) {}

    ///
    #[allow(clippy::unused_self)]
    pub fn anything_pending(&self) -> bool {
        false
    }

    ///
    #[allow(clippy::unused_self)]
    pub fn update_git(&mut self, _ev: AsyncNotification) {
        //
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
        out.push(CommandInfo::new(
            commands::STASHING_SAVE,
            self.visible,
            self.visible || force_all,
        ));

        if self.visible {
            CommandBlocking::Blocking
        } else {
            CommandBlocking::PassingOn
        }
    }

    fn event(&mut self, _ev: crossterm::event::Event) -> bool {
        if self.visible {
            //
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
    }
}
