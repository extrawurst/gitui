use crate::{
    components::{
        dialog_paragraph, utils::time_to_string, CommandBlocking,
        CommandInfo, Component, DrawableComponent, ScrollType,
    },
    keys::SharedKeyConfig,
    strings::{self, order},
    ui::{self, style::SharedTheme},
};
use anyhow::Result;
use asyncgit::{
    sync::{self, CommitDetails, CommitId, CommitMessage},
    CWD,
};
use crossterm::event::{
    Event,
    MouseEvent::{ScrollDown, ScrollUp},
};
use itertools::Itertools;
use std::clone::Clone;
use std::{borrow::Cow, cell::Cell};
use sync::CommitTags;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Span, Spans, Text},
    Frame,
};
enum Detail {
    Author,
    Date,
    Commiter,
    Sha,
}

pub struct DetailsComponent {
    data: Option<CommitDetails>,
    tags: Vec<String>,
    theme: SharedTheme,
    focused: bool,
    current_size: Cell<(u16, u16)>,
    scroll_top: Cell<usize>,
    key_config: SharedKeyConfig,
}

type WrappedCommitMessage<'a> =
    (Vec<Cow<'a, str>>, Vec<Cow<'a, str>>);

impl DetailsComponent {
    ///
    pub const fn new(
        theme: SharedTheme,
        key_config: SharedKeyConfig,
        focused: bool,
    ) -> Self {
        Self {
            data: None,
            tags: Vec::new(),
            theme,
            focused,
            current_size: Cell::new((0, 0)),
            scroll_top: Cell::new(0),
            key_config,
        }
    }

    pub fn set_commit(
        &mut self,
        id: Option<CommitId>,
        tags: Option<CommitTags>,
    ) -> Result<()> {
        self.tags.clear();

        self.data =
            id.and_then(|id| sync::get_commit_details(CWD, id).ok());

        self.scroll_top.set(0);

        if let Some(tags) = tags {
            self.tags.extend(tags)
        }

        Ok(())
    }

    fn wrap_commit_details(
        message: &CommitMessage,
        width: usize,
    ) -> WrappedCommitMessage<'_> {
        let wrapped_title = textwrap::wrap(&message.subject, width);

        if let Some(ref body) = message.body {
            let wrapped_message: Vec<Cow<'_, str>> =
                textwrap::wrap(body, width)
                    .into_iter()
                    .skip(1)
                    .collect();

            (wrapped_title, wrapped_message)
        } else {
            (wrapped_title, vec![])
        }
    }

    fn get_wrapped_lines(
        &self,
        width: usize,
    ) -> WrappedCommitMessage<'_> {
        if let Some(ref data) = self.data {
            if let Some(ref message) = data.message {
                return Self::wrap_commit_details(message, width);
            }
        }

        (vec![], vec![])
    }

    fn get_number_of_lines(&self, width: usize) -> usize {
        let (wrapped_title, wrapped_message) =
            self.get_wrapped_lines(width);

        wrapped_title.len() + wrapped_message.len()
    }

    fn get_theme_for_line(&self, bold: bool) -> Style {
        if bold {
            self.theme.text(true, false).add_modifier(Modifier::BOLD)
        } else {
            self.theme.text(true, false)
        }
    }

    fn get_wrapped_text_message(
        &self,
        width: usize,
        height: usize,
    ) -> Vec<Spans> {
        let (wrapped_title, wrapped_message) =
            self.get_wrapped_lines(width);

        [&wrapped_title[..], &wrapped_message[..]]
            .concat()
            .iter()
            .enumerate()
            .skip(self.scroll_top.get())
            .take(height)
            .map(|(i, line)| {
                Spans::from(vec![Span::styled(
                    line.clone(),
                    self.get_theme_for_line(i < wrapped_title.len()),
                )])
            })
            .collect()
    }

    fn style_detail(&self, field: &Detail) -> Span {
        match field {
            Detail::Author => Span::styled(
                Cow::from(strings::commit::details_author(
                    &self.key_config,
                )),
                self.theme.text(false, false),
            ),
            Detail::Date => Span::styled(
                Cow::from(strings::commit::details_date(
                    &self.key_config,
                )),
                self.theme.text(false, false),
            ),
            Detail::Commiter => Span::styled(
                Cow::from(strings::commit::details_committer(
                    &self.key_config,
                )),
                self.theme.text(false, false),
            ),
            Detail::Sha => Span::styled(
                Cow::from(strings::commit::details_tags(
                    &self.key_config,
                )),
                self.theme.text(false, false),
            ),
        }
    }

    fn get_text_info(&self) -> Vec<Spans> {
        if let Some(ref data) = self.data {
            let mut res = vec![
                Spans::from(vec![
                    self.style_detail(&Detail::Author),
                    Span::styled(
                        Cow::from(format!(
                            "{} <{}>",
                            data.author.name, data.author.email
                        )),
                        self.theme.text(true, false),
                    ),
                ]),
                Spans::from(vec![
                    self.style_detail(&Detail::Date),
                    Span::styled(
                        Cow::from(time_to_string(
                            data.author.time,
                            false,
                        )),
                        self.theme.text(true, false),
                    ),
                ]),
            ];

            if let Some(ref committer) = data.committer {
                res.extend(vec![
                    Spans::from(vec![
                        self.style_detail(&Detail::Commiter),
                        Span::styled(
                            Cow::from(format!(
                                "{} <{}>",
                                committer.name, committer.email
                            )),
                            self.theme.text(true, false),
                        ),
                    ]),
                    Spans::from(vec![
                        self.style_detail(&Detail::Date),
                        Span::styled(
                            Cow::from(time_to_string(
                                committer.time,
                                false,
                            )),
                            self.theme.text(true, false),
                        ),
                    ]),
                ]);
            }

            res.push(Spans::from(vec![
                Span::styled(
                    Cow::from(strings::commit::details_sha(
                        &self.key_config,
                    )),
                    self.theme.text(false, false),
                ),
                Span::styled(
                    Cow::from(data.hash.clone()),
                    self.theme.text(true, false),
                ),
            ]));

            if !self.tags.is_empty() {
                res.push(Spans::from(
                    self.style_detail(&Detail::Sha),
                ));
                res.push(Spans::from(
                    self.tags
                        .iter()
                        .map(|tag| {
                            Span::styled(
                                Cow::from(tag),
                                self.theme.text(true, false),
                            )
                        })
                        .intersperse(Span::styled(
                            Cow::from(","),
                            self.theme.text(true, false),
                        ))
                        .collect::<Vec<Span>>(),
                ));
            }

            res
        } else {
            vec![]
        }
    }

    fn move_scroll_top(&mut self, move_type: ScrollType) -> bool {
        if self.data.is_some() {
            let old = self.scroll_top.get();
            let width = self.current_size.get().0 as usize;
            let height = self.current_size.get().1 as usize;

            let number_of_lines = self.get_number_of_lines(width);

            let max = number_of_lines.saturating_sub(height) as usize;

            let new_scroll_top = match move_type {
                ScrollType::Down => old.saturating_add(1),
                ScrollType::Up => old.saturating_sub(1),
                ScrollType::Home => 0,
                ScrollType::End => max,
                _ => old,
            };

            if new_scroll_top > max {
                return false;
            }

            self.scroll_top.set(new_scroll_top);

            return true;
        }
        false
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
                &strings::commit::details_info_title(
                    &self.key_config,
                ),
                Text::from(self.get_text_info()),
                &self.theme,
                false,
            ),
            chunks[0],
        );

        // We have to take the border into account which is one character on
        // each side.
        let border_width: u16 = 2;

        let width = chunks[1].width.saturating_sub(border_width);
        let height = chunks[1].height.saturating_sub(border_width);

        self.current_size.set((width, height));

        let wrapped_lines = self.get_wrapped_text_message(
            width as usize,
            height as usize,
        );

        f.render_widget(
            dialog_paragraph(
                &strings::commit::details_message_title(
                    &self.key_config,
                ),
                Text::from(wrapped_lines),
                &self.theme,
                self.focused,
            ),
            chunks[1],
        );

        if self.focused {
            ui::draw_scrollbar(
                f,
                chunks[1],
                &self.theme,
                self.get_number_of_lines(width as usize),
                self.scroll_top.get(),
            )
        }

        Ok(())
    }
}

impl Component for DetailsComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        // visibility_blocking(self)

        let width = self.current_size.get().0 as usize;
        let number_of_lines = self.get_number_of_lines(width);

        out.push(
            CommandInfo::new(
                strings::commands::navigate_commit_message(
                    &self.key_config,
                ),
                number_of_lines > 0,
                self.focused || force_all,
            )
            .order(order::NAV),
        );

        CommandBlocking::PassingOn
    }

    fn event(&mut self, event: Event) -> Result<bool> {
        if self.focused {
            if let Event::Mouse(mouse_ev) = event {
                return match mouse_ev {
                    ScrollUp(_col, _row, _key_modifiers) => {
                        self.move_scroll_top(ScrollType::Up)
                    }
                    ScrollDown(_col, _row, _key_modifiers) => {
                        self.move_scroll_top(ScrollType::Down)
                    }
                    _ => Ok(false),
                };
            } else if let Event::Key(e) = event {
                return Ok(if e == self.key_config.move_up {
                    self.move_scroll_top(ScrollType::Up)
                } else if e == self.key_config.move_down {
                    self.move_scroll_top(ScrollType::Down)
                } else if e == self.key_config.home
                    || e == self.key_config.shift_up
                {
                    self.move_scroll_top(ScrollType::Home)
                } else if e == self.key_config.end
                    || e == self.key_config.shift_down
                {
                    self.move_scroll_top(ScrollType::End)
                } else {
                    false
                });
            }
        }

        Ok(false)
    }

    fn focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self, focus: bool) {
        if focus {
            let width = self.current_size.get().0 as usize;
            let height = self.current_size.get().1 as usize;

            self.scroll_top.set(
                self.get_number_of_lines(width)
                    .saturating_sub(height),
            );
        }

        self.focused = focus;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_wrapped_lines(
        message: &CommitMessage,
        width: usize,
    ) -> Vec<Cow<'_, str>> {
        let (wrapped_title, wrapped_message) =
            DetailsComponent::wrap_commit_details(&message, width);

        [&wrapped_title[..], &wrapped_message[..]].concat()
    }

    #[test]
    fn test_textwrap() {
        let message = CommitMessage::from("Commit message");

        assert_eq!(
            get_wrapped_lines(&message, 7),
            vec!["Commit", "message"]
        );
        assert_eq!(
            get_wrapped_lines(&message, 14),
            vec!["Commit message"]
        );

        let message_with_newline =
            CommitMessage::from("Commit message\n");

        assert_eq!(
            get_wrapped_lines(&message_with_newline, 7),
            vec!["Commit", "message"]
        );
        assert_eq!(
            get_wrapped_lines(&message_with_newline, 14),
            vec!["Commit message"]
        );

        let message_with_body = CommitMessage::from(
            "Commit message\n\nFirst line\nSecond line",
        );

        assert_eq!(
            get_wrapped_lines(&message_with_body, 7),
            vec![
                "Commit", "message", "First", "line", "Second",
                "line"
            ]
        );
        assert_eq!(
            get_wrapped_lines(&message_with_body, 14),
            vec!["Commit message", "First line", "Second line"]
        );
    }
}
