use super::{CommandInfo, Component};
use crate::{clear::Clear, git_utils, tui_utils};
use crossterm::event::{Event, KeyCode};
use std::borrow::Cow;
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Text, Widget},
    Frame,
};

#[derive(Default)]
pub struct CommitComponent {
    msg: String,
    // focused: bool,
    visible: bool,
}

impl Component for CommitComponent {
    fn draw<B: Backend>(&self, f: &mut Frame<B>, _rect: Rect) {
        if self.visible {
            let txt = if self.msg.len() > 0 {
                [Text::Raw(Cow::from(self.msg.clone()))]
            } else {
                [Text::Styled(
                    Cow::from("type commit message.."),
                    Style::default().fg(Color::DarkGray),
                )]
            };

            Clear::new(
                Paragraph::new(txt.iter())
                    .block(
                        Block::default()
                            .title("Commit")
                            .borders(Borders::ALL),
                    )
                    .alignment(Alignment::Left),
            )
            .render(f, tui_utils::centered_rect(60, 20, f.size()));
        }
    }

    fn commands(&self) -> Vec<CommandInfo> {
        if !self.visible {
            vec![CommandInfo {
                name: "Commit [c]".to_string(),
                enabled: !git_utils::index_empty(),
            }]
        } else {
            vec![
                CommandInfo {
                    name: "Commit [enter]".to_string(),
                    enabled: self.can_commit(),
                },
                CommandInfo {
                    name: "Close [esc]".to_string(),
                    enabled: true,
                },
            ]
        }
    }

    fn event(&mut self, ev: Event) -> bool {
        if self.visible {
            if let Event::Key(e) = ev {
                return match e.code {
                    KeyCode::Esc => {
                        self.hide();
                        true
                    }
                    KeyCode::Char(c) => {
                        self.msg.push(c);
                        true
                    }
                    KeyCode::Enter if self.can_commit() => {
                        self.commit();
                        true
                    }
                    KeyCode::Backspace if self.msg.len() > 0 => {
                        self.msg.pop().unwrap();
                        true
                    }
                    _ => false,
                };
            }
        } else {
            if ev == Event::Key(KeyCode::Char('c').into()) {
                if !git_utils::index_empty() {
                    self.show();
                    return true;
                }
            }
        }

        false
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hide(&mut self) {
        self.visible = false
    }

    fn show(&mut self) {
        self.visible = true
    }
}

impl CommitComponent {
    fn commit(&mut self) {
        git_utils::commit(&self.msg);
        self.msg.clear();

        self.hide();
    }

    fn can_commit(&self) -> bool {
        self.msg.len() > 0
    }
}
