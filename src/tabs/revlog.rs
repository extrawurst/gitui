use crate::keys;
use asyncgit::{sync, AsyncLog, AsyncNotification, CWD};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use log::debug;
use std::borrow::Cow;
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
}

///
pub struct Revlog {
    scroll: usize,
    items: Vec<LogEntry>,
    git_log: AsyncLog,
}

const STYLE_TIME: Style = Style::new().fg(Color::Blue);
const STYLE_AUTHOR: Style = Style::new().fg(Color::Green);
const STYLE_MSG: Style = Style::new().fg(Color::Reset);

static ELEMENTS_PER_LINE: usize = 6;
static SLICE_SIZE: usize = 500;

impl Revlog {
    ///
    pub fn new(sender: &Sender<AsyncNotification>) -> Self {
        let mut git_log = AsyncLog::new(sender.clone());
        git_log.fetch();
        Self {
            items: Vec::new(),
            git_log,
            scroll: 0,
        }
    }

    ///
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let mut txt = Vec::new();

        for e in &self.items {
            Self::add_entry(e, &mut txt);
        }

        f.render_widget(
            Paragraph::new(
                txt.iter()
                    .skip(self.scroll * ELEMENTS_PER_LINE)
                    .take(area.height as usize * ELEMENTS_PER_LINE),
            )
            .block(Block::default().borders(Borders::ALL))
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
        let commits = sync::get_commits_info(
            CWD,
            self.git_log.get_slice(0, SLICE_SIZE),
        );

        if let Ok(commits) = commits {
            self.items = commits
                .iter()
                .map(|c| LogEntry {
                    author: c.author.clone(),
                    msg: c.message.clone(),
                    time: format!("{}", c.time),
                })
                .collect::<Vec<_>>();
        }
    }

    ///
    pub fn event(&mut self, ev: Event) -> bool {
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

    fn move_selection(&mut self, up: bool) {
        if up {
            self.scroll = self.scroll.saturating_sub(1);
        } else {
            self.scroll = self.scroll.saturating_add(1);
        }

        debug!("move_selection: {}", self.scroll);

        self.update();
    }

    fn add_entry<'a>(e: &'a LogEntry, txt: &mut Vec<Text<'a>>) {
        let count_before = txt.len();

        txt.push(Text::Styled(
            Cow::from(e.time.as_str()),
            STYLE_TIME,
        ));
        txt.push(Text::Raw(Cow::from(" ")));
        txt.push(Text::Styled(
            Cow::from(e.author.as_str()),
            STYLE_AUTHOR,
        ));
        txt.push(Text::Raw(Cow::from(" ")));
        txt.push(Text::Styled(Cow::from(e.msg.as_str()), STYLE_MSG));
        txt.push(Text::Raw(Cow::from("\n")));

        assert_eq!(txt.len() - count_before, ELEMENTS_PER_LINE);
    }
}
