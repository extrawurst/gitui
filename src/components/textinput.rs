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

    /// Clear the `msg`.
    pub fn clear(&mut self) {
        self.msg.clear();
    }

    /// Get the `msg`.
    pub const fn get_text(&self) -> &String {
        &self.msg
    }

    /// Move the cursor right one char.
    fn incr_cursor(&mut self) {
        if let Some(pos) = self.next_char_position() {
            self.cursor_position = pos;
        }
    }

    /// Move the cursor left one char.
    fn decr_cursor(&mut self) {
        let mut new_pos: usize = 0;
        for (bytes, _) in self.msg.char_indices() {
            if bytes >= self.cursor_position {
                break;
            }
            new_pos = bytes;
        }
        self.cursor_position = new_pos;
    }

    /// Get the position of the next char, or, if the cursor points
    /// to the last char, the `msg.len()`.
    /// Returns None when the cursor is already at `msg.len()`.
    fn next_char_position(&self) -> Option<usize> {
        let mut char_indices =
            self.msg[self.cursor_position..].char_indices();
        if char_indices.next().is_some() {
            if let Some((bytes, _)) = char_indices.next() {
                Some(self.cursor_position + bytes)
            } else {
                Some(self.msg.len())
            }
        } else {
            None
        }
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
                // the portion of the text before the cursor is added
                // if the cursor is not at the first character
                if self.cursor_position > 0 {
                    txt.push(Text::styled(
                        &self.msg[..self.cursor_position],
                        Style::default(),
                    ));
                }

                txt.push(Text::styled(
                    if let Some(pos) = self.next_char_position() {
                        &self.msg[self.cursor_position..pos]
                    } else {
                        // if the cursor is at the end of the msg
                        // a whitespace is used to underline
                        " "
                    },
                    Style::default().modifier(Modifier::UNDERLINED),
                ));

                // the final portion of the text is added if there is
                // still remaining characters
                if let Some(pos) = self.next_char_position() {
                    if pos < self.msg.len() {
                        txt.push(Text::styled(
                            &self.msg[pos..],
                            Style::default(),
                        ));
                    }
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
                        self.incr_cursor();
                        return Ok(true);
                    }
                    KeyCode::Delete => {
                        if self.cursor_position < self.msg.len() {
                            self.msg.remove(self.cursor_position);
                        }
                        return Ok(true);
                    }
                    KeyCode::Backspace => {
                        if self.cursor_position > 0 {
                            self.decr_cursor();
                            if self.cursor_position < self.msg.len() {
                            }
                            self.msg.remove(self.cursor_position);
                        }
                        return Ok(true);
                    }
                    KeyCode::Left => {
                        self.decr_cursor();
                        return Ok(true);
                    }
                    KeyCode::Right => {
                        self.incr_cursor();
                        return Ok(true);
                    }
                    KeyCode::Home => {
                        self.cursor_position = 0;
                        return Ok(true);
                    }
                    KeyCode::End => {
                        self.cursor_position = self.msg.len();
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
