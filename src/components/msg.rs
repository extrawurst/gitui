use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent,
};
use crate::{
    keys,
    strings::{self, commands},
    ui,
};
use crossterm::event::Event;
use std::borrow::Cow;
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Text},
    Frame,
};
use ui::style::SharedTheme;

pub struct MsgComponent {
    msg: String,
    visible: bool,
    theme: SharedTheme,
}

use anyhow::Result;

impl DrawableComponent for MsgComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        _rect: Rect,
    ) -> Result<()> {
        if !self.visible {
            return Ok(());
        }
        let txt = vec![Text::Raw(Cow::from(self.msg.as_str()))];

        let area = ui::centered_rect_absolute(65, 25, f.size());
        f.render_widget(Clear, area);
        f.render_widget(
            Paragraph::new(txt.iter())
                .block(
                    Block::default()
                        .title(strings::MSG_TITLE_ERROR)
                        .title_style(self.theme.text_danger())
                        .borders(Borders::ALL)
                        .border_type(BorderType::Thick),
                )
                .alignment(Alignment::Left)
                .wrap(true),
            area,
        );

        Ok(())
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

    fn event(&mut self, ev: Event) -> Result<bool> {
        if self.visible {
            if let Event::Key(e) = ev {
                if let keys::CLOSE_MSG = e {
                    self.hide();
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hide(&mut self) {
        self.visible = false
    }

    fn show(&mut self) -> Result<()> {
        self.visible = true;

        Ok(())
    }
}

impl MsgComponent {
    pub const fn new(theme: SharedTheme) -> Self {
        Self {
            msg: String::new(),
            visible: false,
            theme,
        }
    }
    ///
    pub fn show_msg(&mut self, msg: &str) -> Result<()> {
        self.msg = msg.to_string();
        self.show()?;

        Ok(())
    }
}
