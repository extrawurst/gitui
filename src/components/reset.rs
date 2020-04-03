use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent, EventUpdate,
};
use crate::{
    queue::{InternalEvent, Queue},
    strings, ui,
};

use crossterm::event::{Event, KeyCode};
use std::borrow::Cow;
use strings::commands;
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Text, Widget},
    Frame,
};

pub struct ResetComponent {
    path: String,
    visible: bool,
    queue: Queue,
}

impl DrawableComponent for ResetComponent {
    fn draw<B: Backend>(&self, f: &mut Frame<B>, _rect: Rect) {
        if self.visible {
            let mut txt = Vec::new();
            txt.push(Text::Styled(
                Cow::from(strings::RESET_MSG),
                Style::default().fg(Color::Red),
            ));

            ui::Clear::new(
                Paragraph::new(txt.iter())
                    .block(
                        Block::default()
                            .title(strings::RESET_TITLE)
                            .borders(Borders::ALL),
                    )
                    .alignment(Alignment::Left),
            )
            .render(f, ui::centered_rect(30, 20, f.size()));
        }
    }
}

impl Component for ResetComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        _force_all: bool,
    ) -> CommandBlocking {
        out.push(CommandInfo::new(
            commands::RESET_CONFIRM,
            true,
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
                    KeyCode::Enter => {
                        self.confirm();
                        EventUpdate::None
                    }

                    _ => EventUpdate::None,
                });
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

impl ResetComponent {
    ///
    pub fn new(queue: Queue) -> Self {
        Self {
            path: String::default(),
            visible: false,
            queue,
        }
    }
    ///
    pub fn open_for_path(&mut self, path: &str) {
        self.path = path.to_string();
        self.show();
    }
    ///
    pub fn confirm(&mut self) {
        self.hide();
        self.queue
            .borrow_mut()
            .push_back(InternalEvent::ResetFile(self.path.clone()));
    }
}
