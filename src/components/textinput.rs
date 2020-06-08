use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent,
};
use crate::{strings, ui, ui::style::Theme};
use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use std::borrow::Cow;
use strings::commands;
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Text},
    Frame,
};

/// primarily a subcomponet for user input of text (used in `CommitComponent`)
pub struct TextInputComponent {
    title: String,
    default_msg: String,
    msg: String,
    visible: bool,
    theme: Theme,
}

impl TextInputComponent {
    ///
    pub fn new(
        theme: &Theme,
        title: &str,
        default_msg: &str,
    ) -> Self {
        Self {
            msg: String::default(),
            visible: false,
            theme: *theme,
            title: title.to_string(),
            default_msg: default_msg.to_string(),
        }
    }

    ///
    pub fn clear(&mut self) {
        self.msg.clear();
    }

    ///
    pub const fn get_text(&self) -> &String {
        &self.msg
    }
}

impl DrawableComponent for TextInputComponent {
    fn draw<B: Backend>(
        &mut self,
        f: &mut Frame<B>,
        _rect: Rect,
    ) -> Result<()> {
        if self.visible {
            let txt = if self.msg.is_empty() {
                [Text::Styled(
                    Cow::from(self.default_msg.as_str()),
                    self.theme.text(false, false),
                )]
            } else {
                [Text::Styled(
                    Cow::from(self.msg.clone()),
                    Style::default(),
                )]
            };

            let area = ui::centered_rect(60, 20, f.size());
            f.render_widget(Clear, area);
            f.render_widget(
                Paragraph::new(txt.iter())
                    .block(
                        Block::default()
                            .title(self.title.as_str())
                            .borders(Borders::ALL)
                            .border_type(BorderType::Thick),
                    )
                    .alignment(Alignment::Left),
                area,
            );
        }

        Ok(())
    }
}

impl Component for TextInputComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        _force_all: bool,
    ) -> CommandBlocking {
        out.push(
            CommandInfo::new(
                commands::CLOSE_POPUP,
                true,
                self.visible,
            )
            .order(1),
        );
        visibility_blocking(self)
    }

    fn event(&mut self, ev: Event) -> Result<bool> {
        if self.visible {
            if let Event::Key(e) = ev {
                match e.code {
                    KeyCode::Esc => {
                        self.hide();
                        return Ok(true);
                    }
                    KeyCode::Char(c) => {
                        self.msg.push(c);
                        return Ok(true);
                    }
                    KeyCode::Backspace => {
                        self.msg.pop();
                        return Ok(true);
                    }
                    _ => (),
                };
            }
        }
        Ok(false)
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
