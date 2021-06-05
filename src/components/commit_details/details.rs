use crate::{
    components::{
        dialog_paragraph,
        utils::{scroll_vertical::VerticalScroll, time_to_string},
        CommandBlocking, CommandInfo, Component, DrawableComponent,
        EventState, ScrollType,
    },
    keys::SharedKeyConfig,
    strings::{self, order},
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
    sync::{self, CommitDetails, CommitId, CommitMessage},
    CWD,
};
use crossterm::event::Event;
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
    current_width: Cell<u16>,
    scroll: VerticalScroll,
    scroll_to_bottom_next_draw: Cell<bool>,
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
            scroll_to_bottom_next_draw: Cell::new(false),
            current_width: Cell::new(0),
            scroll: VerticalScroll::new(),
            key_config,
        }
    }

    pub fn set_commit(
        &mut self,
        id: Option<CommitId>,
        tags: Option<CommitTags>,
    ) {
        self.tags.clear();

        self.data =
            id.and_then(|id| sync::get_commit_details(CWD, id).ok());

        self.scroll.reset();

        if let Some(tags) = tags {
            self.tags.extend(tags);
        }
    }

    fn wrap_commit_details(
        message: &CommitMessage,
        width: usize,
    ) -> WrappedCommitMessage<'_> {
        let wrapped_title = textwrap::wrap(&message.subject, width);

        if let Some(ref body) = message.body {
            let wrapped_message: Vec<Cow<'_, str>> =
                textwrap::wrap(body, width).into_iter().collect();

            (wrapped_title, wrapped_message)
        } else {
            (wrapped_title, vec![])
        }
    }

    fn get_wrapped_lines(
        data: &Option<CommitDetails>,
        width: usize,
    ) -> WrappedCommitMessage<'_> {
        if let Some(ref data) = data {
            if let Some(ref message) = data.message {
                return Self::wrap_commit_details(message, width);
            }
        }

        (vec![], vec![])
    }

    fn get_number_of_lines(
        details: &Option<CommitDetails>,
        width: usize,
    ) -> usize {
        let (wrapped_title, wrapped_message) =
            Self::get_wrapped_lines(details, width);

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
            Self::get_wrapped_lines(&self.data, width);

        [&wrapped_title[..], &wrapped_message[..]]
            .concat()
            .iter()
            .enumerate()
            .skip(self.scroll.get_top())
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

    #[allow(unstable_name_collisions)]
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
            self.scroll.move_top(move_type)
        } else {
            false
        }
    }
}

impl DrawableComponent for DetailsComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        const CANSCROLL_STRING: &str = "[\u{2026}]";
        const EMPTY_STRING: &str = "";

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

        self.current_width.set(width);

        let number_of_lines =
            Self::get_number_of_lines(&self.data, usize::from(width));

        self.scroll.update_no_selection(
            number_of_lines,
            usize::from(height),
        );

        if self.scroll_to_bottom_next_draw.get() {
            self.scroll.move_top(ScrollType::End);
            self.scroll_to_bottom_next_draw.set(false);
        }

        let can_scroll = usize::from(height) < number_of_lines;

        f.render_widget(
            dialog_paragraph(
                &format!(
                    "{} {}",
                    strings::commit::details_message_title(
                        &self.key_config,
                    ),
                    if !self.focused && can_scroll {
                        CANSCROLL_STRING
                    } else {
                        EMPTY_STRING
                    }
                ),
                Text::from(self.get_wrapped_text_message(
                    width as usize,
                    height as usize,
                )),
                &self.theme,
                self.focused,
            ),
            chunks[1],
        );

        if self.focused {
            self.scroll.draw(f, chunks[1], &self.theme);
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
        let width = usize::from(self.current_width.get());
        let number_of_lines =
            Self::get_number_of_lines(&self.data, width);

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

    fn event(&mut self, event: Event) -> Result<EventState> {
        if self.focused {
            if let Event::Key(e) = event {
                return Ok(if e == self.key_config.move_up {
                    self.move_scroll_top(ScrollType::Up).into()
                } else if e == self.key_config.move_down {
                    self.move_scroll_top(ScrollType::Down).into()
                } else if e == self.key_config.home
                    || e == self.key_config.shift_up
                {
                    self.move_scroll_top(ScrollType::Home).into()
                } else if e == self.key_config.end
                    || e == self.key_config.shift_down
                {
                    self.move_scroll_top(ScrollType::End).into()
                } else {
                    EventState::NotConsumed
                });
            }
        }

        Ok(EventState::NotConsumed)
    }

    fn focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self, focus: bool) {
        if focus {
            self.scroll_to_bottom_next_draw.set(true);
        } else {
            self.scroll.reset();
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
            "Commit message\nFirst line\nSecond line",
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

#[cfg(test)]
mod test_line_count {
    use super::*;

    #[test]
    fn test_smoke() {
        let commit = CommitDetails {
            message: Some(CommitMessage {
                subject: String::from("subject line"),
                body: Some(String::from("body lone")),
            }),
            ..CommitDetails::default()
        };
        let lines = DetailsComponent::get_number_of_lines(
            &Some(commit.clone()),
            50,
        );
        assert_eq!(lines, 2);

        let lines =
            DetailsComponent::get_number_of_lines(&Some(commit), 8);
        assert_eq!(lines, 4);
    }
}
