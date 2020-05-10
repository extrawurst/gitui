use crate::{
    components::{
        CommandBlocking, CommandInfo, Component, ScrollType,
    },
    keys,
    strings::commands,
};
use asyncgit::{sync, AsyncLog, AsyncNotification, CWD};
use chrono::prelude::*;
use crossbeam_channel::Sender;
use crossterm::event::Event;
use std::{borrow::Cow, cmp, convert::TryFrom, time::Instant};
use sync::{CommitInfo, Tags};
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

impl From<CommitInfo> for LogEntry {
    fn from(c: CommitInfo) -> Self {
        let time =
            DateTime::<Local>::from(DateTime::<Utc>::from_utc(
                NaiveDateTime::from_timestamp(c.time, 0),
                Utc,
            ));
        Self {
            author: c.author,
            msg: c.message,
            time: time.format("%Y-%m-%d %H:%M:%S").to_string(),
            hash: c.hash,
        }
    }
}

const COLOR_SELECTION_BG: Color = Color::Blue;

const STYLE_TAG: Style = Style::new().fg(Color::Yellow);
const STYLE_HASH: Style = Style::new().fg(Color::Magenta);
const STYLE_TIME: Style = Style::new().fg(Color::Blue);
const STYLE_AUTHOR: Style = Style::new().fg(Color::Green);
const STYLE_MSG: Style = Style::new().fg(Color::Reset);

const STYLE_TAG_SELECTED: Style =
    Style::new().fg(Color::Yellow).bg(COLOR_SELECTION_BG);
const STYLE_HASH_SELECTED: Style =
    Style::new().fg(Color::Magenta).bg(COLOR_SELECTION_BG);
const STYLE_TIME_SELECTED: Style =
    Style::new().fg(Color::White).bg(COLOR_SELECTION_BG);
const STYLE_AUTHOR_SELECTED: Style =
    Style::new().fg(Color::Green).bg(COLOR_SELECTION_BG);
const STYLE_MSG_SELECTED: Style =
    Style::new().fg(Color::Reset).bg(COLOR_SELECTION_BG);

static ELEMENTS_PER_LINE: usize = 10;
static SLICE_SIZE: usize = 1000;
static SLICE_OFFSET_RELOAD_THRESHOLD: usize = 100;

///
pub struct Revlog {
    selection: usize,
    selection_max: usize,
    items: Vec<LogEntry>,
    git_log: AsyncLog,
    visible: bool,
    first_open_done: bool,
    scroll_state: (Instant, f32),
    tags: Tags,
}

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
            scroll_state: (Instant::now(), 0_f32),
            tags: Tags::new(),
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
            let tag = if let Some(tag_name) = self.tags.get(&e.hash) {
                tag_name.as_str()
            } else {
                ""
            };
            Self::add_entry(e, idx == selection, &mut txt, tag);
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
        let next_idx = self.items.len();

        let requires_more_data = next_idx
            .saturating_sub(self.selection)
            < SLICE_OFFSET_RELOAD_THRESHOLD;

        self.selection_max = self.git_log.count().saturating_sub(1);

        if requires_more_data {
            let commits = sync::get_commits_info(
                CWD,
                &self.git_log.get_slice(next_idx, SLICE_SIZE),
            );

            if let Ok(commits) = commits {
                self.items
                    .extend(commits.into_iter().map(LogEntry::from));
            }
        }

        if self.tags.is_empty() {
            self.tags = sync::get_tags(CWD).unwrap();
        }
    }

    fn move_selection(&mut self, scroll: ScrollType) {
        self.update_scroll_speed();

        #[allow(clippy::cast_possible_truncation)]
        let speed_int = usize::try_from(self.scroll_state.1 as i64)
            .unwrap()
            .max(1);

        self.selection = match scroll {
            ScrollType::Up => {
                self.selection.saturating_sub(speed_int)
            }
            ScrollType::Down => {
                self.selection.saturating_add(speed_int)
            }
            ScrollType::Home => 0,
            _ => self.selection,
        };

        self.selection = cmp::min(self.selection, self.selection_max);

        self.update();
    }

    fn update_scroll_speed(&mut self) {
        const REPEATED_SCROLL_THRESHOLD_MILLIS: u128 = 300;
        const SCROLL_SPEED_START: f32 = 0.1_f32;
        const SCROLL_SPEED_MAX: f32 = 10_f32;
        const SCROLL_SPEED_MULTIPLIER: f32 = 1.05_f32;

        let now = Instant::now();

        let since_last_scroll =
            now.duration_since(self.scroll_state.0);

        self.scroll_state.0 = now;

        let speed = if since_last_scroll.as_millis()
            < REPEATED_SCROLL_THRESHOLD_MILLIS
        {
            self.scroll_state.1 * SCROLL_SPEED_MULTIPLIER
        } else {
            SCROLL_SPEED_START
        };

        self.scroll_state.1 = speed.min(SCROLL_SPEED_MAX);
    }

    fn add_entry<'a>(
        e: &'a LogEntry,
        selected: bool,
        txt: &mut Vec<Text<'a>>,
        tag: &'a str,
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
            Cow::from(&e.hash[0..7]),
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
        txt.push(splitter.clone());
        txt.push(Text::Styled(
            Cow::from(if tag.is_empty() {
                String::from("")
            } else {
                format!(" {}", tag)
            }),
            if selected {
                STYLE_TAG_SELECTED
            } else {
                STYLE_TAG
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
                    self.move_selection(ScrollType::Up);
                    true
                }
                keys::MOVE_DOWN => {
                    self.move_selection(ScrollType::Down);
                    true
                }
                keys::SHIFT_UP | keys::HOME => {
                    self.move_selection(ScrollType::Home);
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
