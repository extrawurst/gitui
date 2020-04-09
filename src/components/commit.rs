use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent, EventUpdate,
};
use crate::{keys, strings, ui};
use asyncgit::{sync, CWD};
use crossterm::event::{Event, KeyCode};
use log::error;
use std::borrow::Cow;
use strings::commands;
use sync::HookResult;
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
    visible: bool,
    stage_empty: bool,
}

impl DrawableComponent for CommitComponent {
    fn draw<B: Backend>(&self, f: &mut Frame<B>, _rect: Rect) {
        if self.visible {
            let txt = if self.msg.is_empty() {
                [Text::Styled(
                    Cow::from(strings::COMMIT_MSG),
                    Style::default().fg(Color::DarkGray),
                )]
            } else {
                [Text::Raw(Cow::from(self.msg.clone()))]
            };

            ui::Clear::new(
                Paragraph::new(txt.iter())
                    .block(
                        Block::default()
                            .title(strings::COMMIT_TITLE)
                            .borders(Borders::ALL),
                    )
                    .alignment(Alignment::Left),
            )
            .render(f, ui::centered_rect(60, 20, f.size()));
        }
    }
}

impl Component for CommitComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        _force_all: bool,
    ) -> CommandBlocking {
        out.push(CommandInfo::new(
            commands::COMMIT_OPEN,
            !self.stage_empty,
            !self.visible,
        ));
        out.push(CommandInfo::new(
            commands::COMMIT_ENTER,
            self.can_commit(),
            self.visible,
        ));
        out.push(CommandInfo::new(
            commands::CLOSE_POPUP,
            true,
            self.visible,
        ));
        visibility_blocking(self)
    }

    fn event(&mut self, ev: Event) -> Option<EventUpdate> {
        if self.visible {
            if let Event::Key(e) = ev {
                return Some(match e.code {
                    KeyCode::Esc => {
                        self.hide();
                        EventUpdate::Commands
                    }
                    KeyCode::Char(c) => {
                        self.msg.push(c);
                        EventUpdate::Commands
                    }
                    KeyCode::Enter if self.can_commit() => {
                        self.commit();
                        EventUpdate::All
                    }
                    KeyCode::Backspace if !self.msg.is_empty() => {
                        self.msg.pop().unwrap();
                        EventUpdate::Commands
                    }
                    _ => EventUpdate::None,
                });
            }
        } else if let Event::Key(e) = ev {
            if let keys::OPEN_COMMIT = e {
                if !self.stage_empty {
                    self.show();
                    return Some(EventUpdate::All);
                }
            }
        }
        None
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
        if let HookResult::NotOk(e) =
            sync::hooks_commit_msg(CWD, &mut self.msg)
        {
            error!("commit-msg hook error: {}", e);
            return;
        }

        sync::commit(CWD, &self.msg);
        if let HookResult::NotOk(e) = sync::hooks_post_commit(CWD) {
            error!("post-commit hook error: {}", e);
        }

        self.msg.clear();

        self.hide();
    }

    fn can_commit(&self) -> bool {
        !self.msg.is_empty()
    }

    ///
    pub fn set_stage_empty(&mut self, empty: bool) {
        self.stage_empty = empty;
    }
}
