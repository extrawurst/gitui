use super::{
    CommandBlocking, CommandInfo, Component, DrawableComponent,
    EventState,
};
use crate::{
    keys::SharedKeyConfig,
    ui::{self, AsyncSyntaxJob},
};
use anyhow::Result;
use async_utils::AsyncSingleJob;
use asyncgit::{
    sync::{self, TreeFile},
    AsyncNotification, CWD,
};
use crossbeam_channel::Sender;
use itertools::Either;
use std::{convert::From, path::Path};
use tui::{
    backend::Backend,
    layout::Rect,
    text::Text,
    widgets::{Paragraph, Wrap},
    Frame,
};

pub struct SyntaxTextComponent {
    current_file: Option<(String, Either<ui::SyntaxText, String>)>,
    async_highlighting:
        AsyncSingleJob<AsyncSyntaxJob, AsyncNotification>,
    _key_config: SharedKeyConfig,
}

impl SyntaxTextComponent {
    ///
    pub fn new(
        sender: &Sender<AsyncNotification>,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            async_highlighting: AsyncSingleJob::new(
                sender.clone(),
                AsyncNotification::SyntaxHighlighting,
            ),
            current_file: None,
            _key_config: key_config,
        }
    }

    ///
    pub fn update(&mut self, ev: AsyncNotification) {
        if ev == AsyncNotification::SyntaxHighlighting {
            if let Some(job) = self.async_highlighting.get_last() {
                if let Some((path, content)) =
                    self.current_file.as_mut()
                {
                    if let Some(syntax) = (*job.text).clone() {
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
                        Some((path, Either::Right(content)))
                }
                Err(e) => {
                    self.current_file = Some((
                        path,
                        Either::Right(format!(
                            "error loading file: {}",
                            e
                        )),
                    ))
                }
            }
        }
    }
}

impl DrawableComponent for SyntaxTextComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        area: Rect,
    ) -> Result<()> {
        let content =
            Paragraph::new(self.current_file.as_ref().map_or_else(
                || Text::from(""),
                |(_, content)| match content {
                    Either::Left(syn) => syn.into(),
                    Either::Right(s) => Text::from(s.as_str()),
                },
            ))
            .wrap(Wrap { trim: false });
        f.render_widget(content, area);

        Ok(())
    }
}

impl Component for SyntaxTextComponent {
    fn commands(
        &self,
        _out: &mut Vec<CommandInfo>,
        _force_all: bool,
    ) -> CommandBlocking {
        //TODO: scrolling
        CommandBlocking::PassingOn
    }

    fn event(
        &mut self,
        _event: crossterm::event::Event,
    ) -> Result<EventState> {
        //TODO: scrolling
        Ok(EventState::NotConsumed)
    }
}
