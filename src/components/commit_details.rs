use super::{
    dialog_paragraph, utils::time_to_string, DrawableComponent,
};
use crate::{strings, ui::style::Theme};
use anyhow::Result;
use asyncgit::{
    sync::{self, CommitDetails, CommitId},
    AsyncCommitFiles, AsyncNotification, StatusItem, CWD,
};
use crossbeam_channel::Sender;
use std::borrow::Cow;
use sync::Tags;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    widgets::Text,
    Frame,
};

pub struct CommitDetailsComponent {
    data: Option<CommitDetails>,
    tags: Vec<String>,
    files: Option<Vec<StatusItem>>,
    theme: Theme,
    git_commit_files: AsyncCommitFiles,
}

impl DrawableComponent for CommitDetailsComponent {
    fn draw<B: Backend>(
        &mut self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(8),
                    Constraint::Min(10),
                    Constraint::Length(12),
                ]
                .as_ref(),
            )
            .split(rect);

        f.render_widget(
            dialog_paragraph(
                strings::commit::DETAILS_INFO_TITLE,
                self.get_text_info().iter(),
            ),
            chunks[0],
        );

        f.render_widget(
            dialog_paragraph(
                strings::commit::DETAILS_MESSAGE_TITLE,
                self.get_text_message().iter(),
            )
            .wrap(true),
            chunks[1],
        );

        let files_loading = self.files.is_none();
        let files_count = self.files.as_ref().map_or(0, Vec::len);

        let txt = self
            .files
            .as_ref()
            .map_or(vec![], |f| self.get_text_files(f));

        let title = if files_loading {
            strings::commit::DETAILS_FILES_LOADING_TITLE.to_string()
        } else {
            format!(
                "{} {}",
                strings::commit::DETAILS_FILES_TITLE,
                files_count
            )
        };
        f.render_widget(
            dialog_paragraph(title.as_str(), txt.iter()),
            chunks[2],
        );

        Ok(())
    }
}

impl CommitDetailsComponent {
    ///
    pub fn new(
        sender: &Sender<AsyncNotification>,
        theme: &Theme,
    ) -> Self {
        Self {
            theme: *theme,
            data: None,
            tags: Vec::new(),
            files: None,
            git_commit_files: AsyncCommitFiles::new(sender),
        }
    }

    ///
    pub fn set_commit(
        &mut self,
        id: Option<CommitId>,
        tags: &Tags,
    ) -> Result<()> {
        self.data = if let Some(id) = id {
            sync::get_commit_details(CWD, id).ok()
        } else {
            None
        };

        self.tags.clear();
        self.files = None;

        if let Some(id) = id {
            if let Some(tags) = tags.get(&id) {
                self.tags.extend(tags.clone());
            }

            if let Some((fetched_id, res)) =
                self.git_commit_files.current()?
            {
                if fetched_id == id {
                    self.files = Some(res);
                } else {
                    self.git_commit_files.fetch(id)?;
                }
            } else {
                self.git_commit_files.fetch(id)?;
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

    fn get_text_files<'a>(
        &self,
        files: &'a [StatusItem],
    ) -> Vec<Text<'a>> {
        let new_line = Text::Raw(Cow::from("\n"));

        let mut res = Vec::with_capacity(files.len());

        for file in files {
            res.push(Text::Styled(
                Cow::from(file.path.as_str()),
                self.theme.text(true, false),
            ));
            res.push(new_line.clone());
        }

        res
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

    ///
    pub fn any_work_pending(&self) -> bool {
        self.git_commit_files.is_pending()
    }
}
