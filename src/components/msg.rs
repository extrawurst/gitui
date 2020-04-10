use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent, EventUpdate,
};
use crate::{keys, strings, ui};
use crossterm::event::Event;
use std::borrow::Cow;
use strings::commands;
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph, Text, Widget},
    Frame,
};

#[derive(Default)]
pub struct MsgComponent {
    msg: String,
    visible: bool,
}

impl DrawableComponent for MsgComponent {
    fn draw<B: Backend>(&self, f: &mut Frame<B>, _rect: Rect) {
        if self.visible {
            let txt = vec![Text::Raw(Cow::from(self.msg.as_str()))];

            ui::Clear::new(
                Paragraph::new(txt.iter())
                    .block(
                        Block::default()
                            .title(strings::MSG_TITLE)
                            .borders(Borders::ALL),
                    )
                    .wrap(true)
                    .alignment(Alignment::Left),
            )
            .render(f, ui::centered_rect_absolute(65, 25, f.size()));
        }
    }
}

impl Component for MsgComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        _force_all: bool,
    ) -> CommandBlocking {
        out.push(CommandInfo::new(
            commands::CLOSE_MSG,
            true,
            self.visible,
        ));

        visibility_blocking(self)
    }

    fn event(&mut self, ev: Event) -> Option<EventUpdate> {
        if self.visible {
            if let Event::Key(e) = ev {
                if let keys::CLOSE_MSG = e {
                    self.hide();
                }
            }

            Some(EventUpdate::Commands)
        } else {
            None
        }
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

impl MsgComponent {
    ///
    pub fn show_msg(&mut self, msg: &str) {
        self.msg = msg.to_string();
        self.show();
    }
}
