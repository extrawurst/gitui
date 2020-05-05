use crate::{
    components::{CommandBlocking, CommandInfo, Component},
    keys,
    strings::commands,
};
use asyncgit::{sync, AsyncLog, AsyncNotification, CWD};
use chrono::prelude::*;
use crossbeam_channel::Sender;
use crossterm::event::Event;
use std::{borrow::Cow, cmp};
use sync::CommitInfo;
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Text},
    Frame,
};

struct LogEntry {
    time: String,
    author: String,
    msg: String,
    hash: String,
}

impl From<&CommitInfo> for LogEntry {
    fn from(c: &CommitInfo) -> Self {
        let time =
            DateTime::<Local>::from(DateTime::<Utc>::from_utc(
                NaiveDateTime::from_timestamp(c.time, 0),
                Utc,
            ));
        Self {
            author: c.author.clone(),
            msg: c.message.clone(),
            time: time.format("%Y-%m-%d %H:%M:%S").to_string(),
            hash: c.hash[0..7].to_string(),
        }
    }
}

///
pub struct Revlog {
    selection: usize,
    selection_max: usize,
    items: Vec<LogEntry>,
    git_log: AsyncLog,
    visible: bool,
    first_open_done: bool,
}

const COLOR_SELECTION_BG: Color = Color::Blue;

const STYLE_HASH: Style = Style::new().fg(Color::Magenta);
const STYLE_TIME: Style = Style::new().fg(Color::Blue);
const STYLE_AUTHOR: Style = Style::new().fg(Color::Green);
const STYLE_MSG: Style = Style::new().fg(Color::Reset);

const STYLE_HASH_SELECTED: Style =
    Style::new().fg(Color::Magenta).bg(COLOR_SELECTION_BG);
const STYLE_TIME_SELECTED: Style =
    Style::new().fg(Color::White).bg(COLOR_SELECTION_BG);
const STYLE_AUTHOR_SELECTED: Style =
    Style::new().fg(Color::Green).bg(COLOR_SELECTION_BG);
const STYLE_MSG_SELECTED: Style =
    Style::new().fg(Color::Reset).bg(COLOR_SELECTION_BG);

static ELEMENTS_PER_LINE: usize = 8;
static SLICE_SIZE: usize = 300;
static SLICE_OFFSET_RELOAD_THRESHOLD: usize = 100;

impl Revlog {
    ///
    pub fn new(sender: &Sender<AsyncNotification>) -> Self {
        Self {
            items: Vec::new(),
            git_log: AsyncLog::new(sender.clone()),
            selection: 0,
            selection_max: 0,
            visible: false,
            first_open_done: false,
        }
    }

    ///
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let height = area.height as usize;
        let selection = self.selection;
        let height_d2 = height as usize / 2;
        let min = selection.saturating_sub(height_d2);

        let mut txt = Vec::new();
        for (idx, e) in self.items.iter().enumerate() {
            Self::add_entry(e, idx == selection, &mut txt);
        }

        let title =
            format!("commit {}/{}", selection, self.selection_max);

        f.render_widget(
            Paragraph::new(
                txt.iter()
                    .skip(min * ELEMENTS_PER_LINE)
                    .take(height * ELEMENTS_PER_LINE),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title.as_str()),
            )
            .alignment(Alignment::Left),
            area,
        );
    }

    ///
    pub fn any_work_pending(&self) -> bool {
        self.git_log.is_pending()
    }

    ///
    pub fn update(&mut self) {
        let max_idx = if self.items.is_empty() {
            0
        } else {
            self.items.len() - 1
        };

        let requires_more_data = max_idx
            .saturating_sub(self.selection)
            < SLICE_OFFSET_RELOAD_THRESHOLD;

        self.selection_max = self.git_log.count().saturating_sub(1);

        if requires_more_data {
            let commits = sync::get_commits_info(
                CWD,
                self.git_log.get_slice(max_idx + 1, SLICE_SIZE),
            );

            if let Ok(commits) = commits {
                self.items.extend(
                    commits.iter().map(|c| LogEntry::from(c)),
                );
            }
        }
    }

    fn move_selection(&mut self, up: bool) {
        if up {
            self.selection = self.selection.saturating_sub(1);
        } else {
            self.selection = self.selection.saturating_add(1);
        }

        self.selection = cmp::min(self.selection, self.selection_max);

        self.update();
    }

    fn add_entry<'a>(
        e: &'a LogEntry,
        selected: bool,
        txt: &mut Vec<Text<'a>>,
    ) {
        let count_before = txt.len();

        let splitter_txt = Cow::from(" ");
        let splitter = if selected {
            Text::Styled(
                splitter_txt,
                Style::new().bg(COLOR_SELECTION_BG),
            )
        } else {
            Text::Raw(splitter_txt)
        };

        txt.push(Text::Styled(
            Cow::from(e.hash.as_str()),
            if selected {
                STYLE_HASH_SELECTED
            } else {
                STYLE_HASH
            },
        ));
        txt.push(splitter.clone());
        txt.push(Text::Styled(
            Cow::from(e.time.as_str()),
            if selected {
                STYLE_TIME_SELECTED
            } else {
                STYLE_TIME
            },
        ));
        txt.push(splitter.clone());
        txt.push(Text::Styled(
            Cow::from(e.author.as_str()),
            if selected {
                STYLE_AUTHOR_SELECTED
            } else {
                STYLE_AUTHOR
            },
        ));
        txt.push(splitter);
        txt.push(Text::Styled(
            Cow::from(e.msg.as_str()),
            if selected {
                STYLE_MSG_SELECTED
            } else {
                STYLE_MSG
            },
        ));
        txt.push(Text::Raw(Cow::from("\n")));

        assert_eq!(txt.len() - count_before, ELEMENTS_PER_LINE);
    }
}

impl Component for Revlog {
    fn event(&mut self, ev: Event) -> bool {
        if let Event::Key(k) = ev {
            return match k {
                keys::MOVE_UP => {
                    self.move_selection(true);
                    true
                }
                keys::MOVE_DOWN => {
                    self.move_selection(false);
                    true
                }
                _ => false,
            };
        }

        false
    }

    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        out.push(CommandInfo::new(
            commands::SCROLL,
            self.visible,
            self.visible || force_all,
        ));

        CommandBlocking::PassingOn
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn show(&mut self) {
        self.visible = true;

        if !self.first_open_done {
            self.first_open_done = true;
            self.git_log.fetch();
        }
    }
}
