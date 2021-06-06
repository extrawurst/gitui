use super::{
    CommandBlocking, CommandInfo, Component, DrawableComponent,
    EventState,
};
use crate::{
    keys::SharedKeyConfig,
    strings,
    ui::{
        self, common_nav, style::SharedTheme, AsyncSyntaxJob,
        ParagraphState, ScrollPos, StatefulParagraph,
    },
};
use anyhow::Result;
use asyncgit::{
    asyncjob::AsyncSingleJob,
    sync::{self, TreeFile},
    AsyncNotification, CWD,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use filetreelist::MoveSelection;
use itertools::Either;
use std::{cell::Cell, convert::From, path::Path};
use tui::{
    backend::Backend,
    layout::Rect,
    text::Text,
    widgets::{Block, Borders, Wrap},
    Frame,
};

pub struct SyntaxTextComponent {
    current_file: Option<(String, Either<ui::SyntaxText, String>)>,
    async_highlighting:
        AsyncSingleJob<AsyncSyntaxJob, AsyncNotification>,
    key_config: SharedKeyConfig,
    paragraph_state: Cell<ParagraphState>,
    focused: bool,
    theme: SharedTheme,
}

impl SyntaxTextComponent {
    ///
    pub fn new(
        sender: &Sender<AsyncNotification>,
        key_config: SharedKeyConfig,
        theme: SharedTheme,
    ) -> Self {
        Self {
            async_highlighting: AsyncSingleJob::new(
                sender.clone(),
                AsyncNotification::SyntaxHighlighting,
            ),
            current_file: None,
            paragraph_state: Cell::new(ParagraphState::default()),
            focused: false,
            key_config,
            theme,
        }
    }

    ///
    pub fn update(&mut self, ev: AsyncNotification) {
        if ev == AsyncNotification::SyntaxHighlighting {
            if let Some(job) = self.async_highlighting.take_last() {
                if let Some((path, content)) =
                    self.current_file.as_mut()
                {
                    if let Some(syntax) = job.result() {
                        if syntax.path() == Path::new(path) {
                            *content = Either::Left(syntax);
                        }
                    }
                }
            }
        }
    }

    ///
    pub fn any_work_pending(&self) -> bool {
        self.async_highlighting.is_pending()
    }

    ///
    pub fn clear(&mut self) {
        self.current_file = None;
    }

    ///
    pub fn load_file(&mut self, path: String, item: &TreeFile) {
        let already_loaded = self
            .current_file
            .as_ref()
            .map(|(current_file, _)| current_file == &path)
            .unwrap_or_default();

        if !already_loaded {
            //TODO: fetch file content async aswell
            match sync::tree_file_content(CWD, item) {
                Ok(content) => {
                    self.async_highlighting.spawn(
                        AsyncSyntaxJob::new(
                            content.clone(),
                            path.clone(),
                        ),
                    );

                    self.current_file =
                        Some((path, Either::Right(content)));
                }
                Err(e) => {
                    self.current_file = Some((
                        path,
                        Either::Right(format!(
                            "error loading file: {}",
                            e
                        )),
                    ));
                }
            }
        }
    }

    fn scroll(&self, nav: MoveSelection) -> bool {
        let state = self.paragraph_state.get();

        let new_scroll_pos = match nav {
            MoveSelection::Down => state.scroll().y.saturating_add(1),
            MoveSelection::Up => state.scroll().y.saturating_sub(1),
            MoveSelection::Top => 0,
            MoveSelection::End => state
                .lines()
                .saturating_sub(state.height().saturating_sub(2)),
            MoveSelection::PageUp => state
                .scroll()
                .y
                .saturating_sub(state.height().saturating_sub(2)),
            MoveSelection::PageDown => state
                .scroll()
                .y
                .saturating_add(state.height().saturating_sub(2)),
            _ => state.scroll().y,
        };

        self.set_scroll(new_scroll_pos)
    }

    fn set_scroll(&self, pos: u16) -> bool {
        let mut state = self.paragraph_state.get();

        let new_scroll_pos = pos.min(
            state
                .lines()
                .saturating_sub(state.height().saturating_sub(2)),
        );

        if new_scroll_pos == state.scroll().y {
            return false;
        }

        state.set_scroll(ScrollPos {
            x: 0,
            y: new_scroll_pos,
        });
        self.paragraph_state.set(state);

        true
    }
}

impl DrawableComponent for SyntaxTextComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        area: Rect,
    ) -> Result<()> {
        let text = self.current_file.as_ref().map_or_else(
            || Text::from(""),
            |(_, content)| match content {
                Either::Left(syn) => syn.into(),
                Either::Right(s) => Text::from(s.as_str()),
            },
        );

        let content = StatefulParagraph::new(text)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title(
                        self.current_file
                            .as_ref()
                            .map(|(name, _)| name.clone())
                            .unwrap_or_default(),
                    )
                    .borders(Borders::ALL)
                    .border_style(self.theme.title(self.focused())),
            );

        let mut state = self.paragraph_state.get();

        f.render_stateful_widget(content, area, &mut state);

        self.paragraph_state.set(state);

        self.set_scroll(state.scroll().y);

        if self.focused() {
            ui::draw_scrollbar(
                f,
                area,
                &self.theme,
                usize::from(state.lines().saturating_sub(
                    state.height().saturating_sub(2),
                )),
                usize::from(state.scroll().y),
            );
        }

        Ok(())
    }
}

impl Component for SyntaxTextComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.focused() || force_all {
            out.push(
                CommandInfo::new(
                    strings::commands::scroll(&self.key_config),
                    true,
                    true,
                )
                .order(strings::order::NAV),
            );
        }
        CommandBlocking::PassingOn
    }

    fn event(
        &mut self,
        event: crossterm::event::Event,
    ) -> Result<EventState> {
        if let Event::Key(key) = event {
            if let Some(nav) = common_nav(key, &self.key_config) {
                return Ok(self
                    .scroll(nav)
                    .then(|| EventState::Consumed)
                    .unwrap_or(EventState::NotConsumed));
            }
        }

        Ok(EventState::NotConsumed)
    }

    ///
    fn focused(&self) -> bool {
        self.focused
    }

    /// focus/unfocus this component depending on param
    fn focus(&mut self, focus: bool) {
        self.focused = focus;
    }
}
