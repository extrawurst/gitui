use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent,
};
use crate::{
    components::dialog_paragraph, strings, ui, ui::style::Theme,
};
use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use strings::commands;
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Clear, Text},
    Frame,
};

/// primarily a subcomponet for user input of text (used in `CommitComponent`)
pub struct TextInputComponent {
    title: String,
    default_msg: String,
    msg: String,
    visible: bool,
    theme: Theme,
    cursor_position: usize,
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
            cursor_position: 0,
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

    fn incr_cursor(&mut self, amt: usize) {
        self.cursor_position = self
            .cursor_position
            .saturating_add(amt)
            .min(self.msg.len());
    }

    fn decr_cursor(&mut self, amt: usize) {
        self.cursor_position =
            self.cursor_position.saturating_sub(amt);
    }
}

impl DrawableComponent for TextInputComponent {
    fn draw<B: Backend>(
        &mut self,
        f: &mut Frame<B>,
        _rect: Rect,
    ) -> Result<()> {
        if self.visible {
            let mut txt: Vec<tui::widgets::Text> = Vec::new();
            if self.msg.is_empty() {
                txt.push(Text::styled(
                    self.default_msg.as_str(),
                    self.theme.text(false, false),
                ));
            } else {
                let len = self.msg.len();

                // the portion of the text before the cursor is added
                // if the cursor is not at the first character
                if self.cursor_position > 0 {
                    txt.push(Text::styled(
                        &self.msg[..self.cursor_position],
                        Style::default(),
                    ));
                }

                txt.push(Text::styled(
                    if self.cursor_position == len {
                        // if the cursor is at the end of the text, a
                        // trailing space is appended to underline
                        " "
                    } else {
                        // otherwise the character the cursor is at is
                        // underlined
                        &self.msg[self.cursor_position
                            ..=self.cursor_position]
                    },
                    Style::default().modifier(Modifier::UNDERLINED),
                ));

                // the final portion of the text is added if there is
                // still remaining characters
                if self.cursor_position < len - 1 {
                    txt.push(Text::styled(
                        &self.msg[self.cursor_position + 1..],
                        Style::default(),
                    ));
                }
            };

            let area = ui::centered_rect(60, 20, f.size());
            f.render_widget(Clear, area);
            f.render_widget(
                dialog_paragraph(self.title.as_str(), txt.iter()),
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
                        self.msg.insert(self.cursor_position, c);
                        self.incr_cursor(1);
                        return Ok(true);
                    }
                    KeyCode::Delete => {
                        if self.cursor_position < self.msg.len() {
                            self.msg.remove(self.cursor_position);
                        }
                        return Ok(true);
                    }
                    KeyCode::Backspace => {
                        if 0 < self.cursor_position
                            && self.cursor_position <= self.msg.len()
                        {
                            self.msg.remove(self.cursor_position - 1);
                        }
                        self.decr_cursor(1);
                        return Ok(true);
                    }
                    KeyCode::Left => {
                        self.decr_cursor(1);
                        return Ok(true);
                    }
                    KeyCode::Right => {
                        self.incr_cursor(1);
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
