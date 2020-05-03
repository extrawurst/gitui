use asyncgit::{sync, AsyncLog, AsyncNotification, CWD};
use crossbeam_channel::Sender;
use std::borrow::Cow;
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Text},
    Frame,
};

pub struct LogEntry {
    time: String,
    author: String,
    msg: String,
}

pub struct Revlog {
    items: Vec<LogEntry>,
    git_log: AsyncLog,
}

const STYLE_TIME: Style = Style::new().fg(Color::Blue);
const STYLE_AUTHOR: Style = Style::new().fg(Color::Green);
const STYLE_MSG: Style = Style::new().fg(Color::Reset);

impl Revlog {
    ///
    pub fn new(sender: &Sender<AsyncNotification>) -> Self {
        let mut git_log = AsyncLog::new(sender.clone());
        git_log.fetch();
        Self {
            items: Vec::new(),
            git_log,
        }
    }

    ///
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let mut txt = Vec::new();

        for e in &self.items {
            Self::add_entry(e, &mut txt);
        }

        f.render_widget(
            Paragraph::new(txt.iter())
                .block(
                    Block::default()
                        .title("log")
                        .borders(Borders::ALL),
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
        let commits =
            sync::get_commits_info(CWD, self.git_log.get_slice(1000));

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

        // debug!(
        //     "log: {} ({})",
        //     self.git_log.count(),
        //     self.git_log.is_pending()
        // );
    }

    fn add_entry<'a>(e: &'a LogEntry, txt: &mut Vec<Text<'a>>) {
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
    }
}
