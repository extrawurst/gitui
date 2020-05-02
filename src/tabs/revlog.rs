use asyncgit::{sync, CWD};
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

pub struct LogChunk {
    items: Vec<LogEntry>,
}

pub struct Revlog {
    chunk: LogChunk,
}

const STYLE_TIME: Style = Style::new().fg(Color::Blue);
const STYLE_AUTHOR: Style = Style::new().fg(Color::Green);
const STYLE_MSG: Style = Style::new().fg(Color::Reset);

impl Revlog {
    ///
    pub fn new() -> Self {
        let items = sync::get_log(CWD, 100)
            .unwrap()
            .iter()
            .map(|e| LogEntry {
                author: e.author.clone(),
                msg: e.message.clone(),
                time: format!("{}", e.time),
            })
            .collect::<Vec<_>>();
        Self {
            chunk: LogChunk { items },
        }
    }

    ///
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let mut txt = Vec::new();

        for e in &self.chunk.items {
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
