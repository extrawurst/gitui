use crate::{
    components::{
        dialog_paragraph, utils::time_to_string, CommandBlocking,
        CommandInfo, Component, DrawableComponent,
    },
    strings,
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
    sync::{self, CommitDetails, CommitId, Tags},
    CWD,
};
use crossterm::event::Event;
use std::borrow::Cow;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    widgets::Text,
    Frame,
};

pub struct DetailsComponent {
    data: Option<CommitDetails>,
    tags: Vec<String>,
    theme: SharedTheme,
}

impl DetailsComponent {
    ///
    pub const fn new(theme: SharedTheme) -> Self {
        Self {
            data: None,
            tags: Vec::new(),
            theme,
        }
    }

    pub fn set_commit(
        &mut self,
        id: Option<CommitId>,
        tags: Option<&Tags>,
    ) -> Result<()> {
        self.tags.clear();

        self.data = if let Some(id) = id {
            sync::get_commit_details(CWD, id).ok()
        } else {
            None
        };

        if let Some(id) = id {
            if let Some(tags) = tags {
                if let Some(tags) = tags.get(&id) {
                    self.tags.extend(tags.clone());
                }
            }
        }

        Ok(())
    }

    fn get_text_message(&self) -> Vec<Text> {
        if let Some(ref data) = self.data {
            if let Some(ref message) = data.message {
                let mut res = vec![Text::Styled(
                    Cow::from(message.subject.clone()),
                    self.theme
                        .text(true, false)
                        .modifier(Modifier::BOLD),
                )];

                if let Some(ref body) = message.body {
                    res.push(Text::Styled(
                        Cow::from(body),
                        self.theme.text(true, false),
                    ));
                }

                return res;
            }
        }
        vec![]
    }

    fn get_text_info(&self) -> Vec<Text> {
        let new_line = Text::Raw(Cow::from("\n"));

        if let Some(ref data) = self.data {
            let mut res = vec![
                Text::Styled(
                    Cow::from(strings::commit::DETAILS_AUTHOR),
                    self.theme.text(false, false),
                ),
                Text::Styled(
                    Cow::from(format!(
                        "{} <{}>",
                        data.author.name, data.author.email
                    )),
                    self.theme.text(true, false),
                ),
                new_line.clone(),
                Text::Styled(
                    Cow::from(strings::commit::DETAILS_DATE),
                    self.theme.text(false, false),
                ),
                Text::Styled(
                    Cow::from(time_to_string(
                        data.author.time,
                        false,
                    )),
                    self.theme.text(true, false),
                ),
                new_line.clone(),
            ];

            if let Some(ref committer) = data.committer {
                res.extend(vec![
                    Text::Styled(
                        Cow::from(strings::commit::DETAILS_COMMITTER),
                        self.theme.text(false, false),
                    ),
                    Text::Styled(
                        Cow::from(format!(
                            "{} <{}>",
                            committer.name, committer.email
                        )),
                        self.theme.text(true, false),
                    ),
                    new_line.clone(),
                    Text::Styled(
                        Cow::from(strings::commit::DETAILS_DATE),
                        self.theme.text(false, false),
                    ),
                    Text::Styled(
                        Cow::from(time_to_string(
                            committer.time,
                            false,
                        )),
                        self.theme.text(true, false),
                    ),
                    new_line.clone(),
                ]);
            }

            res.extend(vec![
                Text::Styled(
                    Cow::from(strings::commit::DETAILS_SHA),
                    self.theme.text(false, false),
                ),
                Text::Styled(
                    Cow::from(data.hash.clone()),
                    self.theme.text(true, false),
                ),
                new_line.clone(),
            ]);

            if !self.tags.is_empty() {
                res.push(Text::Styled(
                    Cow::from(strings::commit::DETAILS_TAGS),
                    self.theme.text(false, false),
                ));

                for tag in &self.tags {
                    res.push(Text::Styled(
                        Cow::from(tag),
                        self.theme.text(true, false),
                    ));
                }
            }

            res
        } else {
            vec![]
        }
    }
}

impl DrawableComponent for DetailsComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [Constraint::Length(8), Constraint::Min(10)].as_ref(),
            )
            .split(rect);

        f.render_widget(
            dialog_paragraph(
                strings::commit::DETAILS_INFO_TITLE,
                self.get_text_info().iter(),
                &self.theme,
                false,
            ),
            chunks[0],
        );

        f.render_widget(
            dialog_paragraph(
                strings::commit::DETAILS_MESSAGE_TITLE,
                self.get_text_message().iter(),
                &self.theme,
                false,
            )
            .wrap(true),
            chunks[1],
        );

        Ok(())
    }
}

impl Component for DetailsComponent {
    fn commands(
        &self,
        _out: &mut Vec<CommandInfo>,
        _force_all: bool,
    ) -> CommandBlocking {
        // visibility_blocking(self)
        CommandBlocking::PassingOn
    }

    fn event(&mut self, _ev: Event) -> Result<bool> {
        Ok(false)
    }
}
