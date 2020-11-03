use crate::ui::Size;
use crate::{
    components::{
        popup_paragraph, visibility_blocking, CommandBlocking,
        CommandInfo, Component, DrawableComponent,
    },
    keys::SharedKeyConfig,
    strings,
    ui::{self, style::SharedTheme},
};
use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use itertools::Itertools;
use std::cell::Cell;
use std::ops::Range;
use tui::{
    backend::Backend, layout::Rect, style::Modifier, text::Span,
    text::Spans, widgets::Clear, Frame,
};

#[derive(PartialEq)]
pub enum InputType {
    Singleline,
    Multiline,
    Password,
}

/// primarily a subcomponet for user input of text (used in `CommitComponent`)
pub struct TextInputComponent {
    title: String,
    default_msg: String,
    msg: Vec<String>,
    visible: bool,
    theme: SharedTheme,
    key_config: SharedKeyConfig,
    cursor_position: usize,
    current_line: usize,
    input_type: InputType,
    num_lines: Cell<usize>,
    display_off: usize,
}

impl TextInputComponent {
    ///
    pub fn new(
        theme: SharedTheme,
        key_config: SharedKeyConfig,
        title: &str,
        default_msg: &str,
    ) -> Self {
        Self {
            msg: vec![String::default()],
            visible: false,
            theme,
            key_config,
            title: title.to_string(),
            default_msg: default_msg.to_string(),
            cursor_position: 0,
            current_line: 0,
            input_type: InputType::Multiline,
            // tests need a few lines
            num_lines: Cell::new(5),
            display_off: 0,
        }
    }

    pub const fn with_input_type(
        mut self,
        input_type: InputType,
    ) -> Self {
        self.input_type = input_type;
        self
    }

    /// Clear the `msg`.
    pub fn clear(&mut self) {
        self.msg = vec![String::default()];
        self.cursor_position = 0;
        self.current_line = 0;
    }

    /// Get the `msg`.
    pub fn get_text(&self) -> String {
        self.msg.join("\n")
    }

    /// Move the cursor right one char.
    fn incr_cursor(&mut self) {
        if let Some(pos) = self.next_char_position() {
            self.cursor_position = pos;
        } else {
            self.next_line(0);
        }
    }
    fn prev_line(&mut self, cursor: usize) {
        if self.current_line > 0 {
            self.current_line -= 1;
            self.cursor_position = std::cmp::min(
                cursor,
                self.msg[self.current_line].len(),
            );
            if self.current_line == self.display_off
                && self.display_off > 0
            {
                self.display_off -= 1;
            }
        }
    }
    fn next_line(&mut self, cursor: usize) {
        if self.current_line < self.msg.len() - 1 {
            self.current_line += 1;
            self.cursor_position = std::cmp::min(
                cursor,
                self.msg[self.current_line].len(),
            );
            if self.current_line > self.num_lines.get() - 1 {
                self.display_off += 1;
            }
        }
    }
    /// Move the cursor left one char.
    fn decr_cursor(&mut self) {
        if self.cursor_position == 0 {
            self.prev_line(std::usize::MAX);
        } else {
            let mut index = self.cursor_position.saturating_sub(1);
            while index > 0
                && !self.msg[self.current_line]
                    .is_char_boundary(index)
            {
                index -= 1;
            }
            self.cursor_position = index;
        }
    }

    /// Get the position of the next char, or, if the cursor points
    /// to the last char, the `msg.len()`.
    /// Returns None when the cursor is already at `msg.len()`.
    fn next_char_position(&self) -> Option<usize> {
        if self.cursor_position >= self.msg[self.current_line].len() {
            return None;
        }
        let mut index = self.cursor_position.saturating_add(1);
        while index < self.msg.len()
            && !self.msg[self.current_line].is_char_boundary(index)
        {
            index += 1;
        }
        Some(index)
    }

    fn line_up(&mut self) {
        self.prev_line(self.cursor_position);
    }
    fn line_down(&mut self) {
        self.next_line(self.cursor_position);
    }

    fn split_line(&mut self) {
        self.msg[self.current_line]
            .insert(self.cursor_position, '\n');
        let cl = self.current_line;
        self.set_text(&self.get_text());
        self.current_line = cl;
        self.next_line(0);
    }
    fn merge_lines(&mut self) {
        let next_line = self.msg[self.current_line + 1].clone();
        self.msg.remove(self.current_line + 1);
        self.msg[self.current_line].push_str(&next_line);
    }
    fn delete_char(&mut self) {
        if self.cursor_position < self.msg[self.current_line].len() {
            self.msg[self.current_line].remove(self.cursor_position);
        } else if self.msg.len() + 1 > self.current_line {
            self.merge_lines();
        }
    }
    fn backspace(&mut self) {
        if self.cursor_position > 0 {
            self.decr_cursor();
            self.msg[self.current_line].remove(self.cursor_position);
        } else if self.current_line > 0 {
            self.prev_line(std::usize::MAX);
            self.merge_lines();
        }
    }

    /// Set the `msg`.
    pub fn set_text(&mut self, msg: &str) {
        self.msg = msg.split('\n').map(ToString::to_string).collect();
        self.cursor_position = 0;
        self.current_line = 0;
    }

    /// Set the `title`.
    pub fn set_title(&mut self, t: String) {
        self.title = t;
    }

    fn get_draw_text(&self) -> Vec<Spans> {
        let style = self.theme.text(true, false);
        let mut spans = Vec::new();

        for i in self.display_off
            ..std::cmp::min(
                self.num_lines.get() + self.display_off,
                self.msg.len(),
            )
        {
            let mut txt = Vec::new();

            if i == self.current_line {
                // The portion of the text before the cursor is added
                // if the cursor is not at the first character.
                if self.cursor_position > 0 {
                    txt.push(Span::styled(
                        self.get_msg(0..self.cursor_position),
                        style,
                    ));
                }

                // this code with _ for the trailing cursor character needs to be revised once tui fixes
                // https://github.com/fdehau/tui-rs/issues/404
                // it should be NBSP => const NBSP: &str = "\u{00a0}";
                // ...
                let cursor_str = self
                    .next_char_position()
                    // if the cursor is at the end of the msg
                    // a whitespace is used to underline
                    .map_or("_".to_owned(), |pos| {
                        self.get_msg(self.cursor_position..pos)
                    });

                if cursor_str == "\n" {
                    txt.push(Span::styled(
                        "\u{21b5}",
                        self.theme
                            .text(false, false)
                            .add_modifier(Modifier::UNDERLINED),
                    ));
                }
                // ... and this conditional underline needs to be removed

                if cursor_str == "_" {
                    txt.push(Span::styled(cursor_str, style));
                } else {
                    txt.push(Span::styled(
                        cursor_str,
                        style.add_modifier(Modifier::UNDERLINED),
                    ));
                }
                // The final portion of the text is added if there are
                // still remaining characters.
                if let Some(pos) = self.next_char_position() {
                    if pos < self.msg[i].len() {
                        txt.push(Span::styled(
                            self.get_msg(pos..self.msg[i].len()),
                            style,
                        ));
                    }
                }
            } else {
                txt = vec![Span::raw(self.msg[i].clone())];
            }
            spans.push(Spans::from(txt));
        }
        spans
    }

    fn get_msg(&self, range: Range<usize>) -> String {
        match self.input_type {
            InputType::Password => range.map(|_| "*").join(""),
            _ => self.msg[self.current_line][range].to_owned(),
        }
    }
}

impl DrawableComponent for TextInputComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        _rect: Rect,
    ) -> Result<()> {
        if self.visible {
            let txt = if self.msg.is_empty() {
                vec![Spans::from(Span::styled(
                    self.default_msg.as_str(),
                    self.theme.text(false, false),
                ))]
            } else {
                self.get_draw_text()
            };

            let area = match self.input_type {
                InputType::Multiline => {
                    let area = ui::centered_rect(60, 20, f.size());
                    ui::rect_inside(
                        Size::new(10, 3),
                        f.size().into(),
                        area,
                    )
                }
                _ => ui::centered_rect_absolute(32, 3, f.size()),
            };
            self.num_lines.set(area.height as usize - 2);
            f.render_widget(Clear, area);
            f.render_widget(
                popup_paragraph(
                    self.title.as_str(),
                    txt,
                    &self.theme,
                    true,
                ),
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
                strings::commands::close_popup(&self.key_config),
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
                let is_ctrl =
                    e.modifiers.contains(KeyModifiers::CONTROL);
                if (e.code == KeyCode::Enter
                    || e.code == KeyCode::Char('j'))
                    && is_ctrl
                {
                    self.split_line();
                    return Ok(true);
                }
                if e == self.key_config.exit_popup {
                    self.hide();
                    return Ok(true);
                }

                match e.code {
                    KeyCode::Char(c) if !is_ctrl => {
                        self.msg[self.current_line]
                            .insert(self.cursor_position, c);
                        self.incr_cursor();
                        return Ok(true);
                    }
                    KeyCode::Delete => {
                        self.delete_char();
                        return Ok(true);
                    }
                    KeyCode::Backspace => {
                        self.backspace();
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
                    KeyCode::Up => {
                        self.line_up();
                        return Ok(true);
                    }
                    KeyCode::Down => {
                        self.line_down();
                        return Ok(true);
                    }

                    KeyCode::Home => {
                        self.cursor_position = 0;
                        return Ok(true);
                    }
                    KeyCode::End => {
                        self.cursor_position =
                            self.msg[self.current_line].len();
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

#[cfg(test)]
mod tests {
    use super::*;
    use tui::style::Style;

    #[test]
    fn test_smoke() {
        let mut comp = TextInputComponent::new(
            SharedTheme::default(),
            SharedKeyConfig::default(),
            "",
            "",
        );

        comp.set_text("a\nb");

        assert_eq!(comp.cursor_position, 0);

        comp.incr_cursor();
        assert_eq!(comp.cursor_position, 1);

        comp.decr_cursor();
        assert_eq!(comp.cursor_position, 0);
    }

    #[test]
    fn text_cursor_initial_position() {
        let mut comp = TextInputComponent::new(
            SharedTheme::default(),
            SharedKeyConfig::default(),
            "",
            "",
        );
        let theme = SharedTheme::default();
        let underlined = theme
            .text(true, false)
            .add_modifier(Modifier::UNDERLINED);

        comp.set_text("a");

        let txt = &comp.get_draw_text()[0];

        assert_eq!(txt.width(), 1);
        assert_eq!(get_text(&txt.0[0]), Some("a"));
        assert_eq!(get_style(&txt.0[0]), Some(&underlined));
    }

    #[test]
    fn test_cursor_second_position() {
        let mut comp = TextInputComponent::new(
            SharedTheme::default(),
            SharedKeyConfig::default(),
            "",
            "",
        );
        let theme = SharedTheme::default();

        // retained for when tui trailing NBSP bug fixed
        let _underlined = theme
            .text(true, false)
            .add_modifier(Modifier::UNDERLINED);

        let not_underlined = Style::default();

        comp.set_text("a");
        comp.incr_cursor();

        let txt = &comp.get_draw_text()[0];

        assert_eq!(txt.width(), 2);
        assert_eq!(get_text(&txt.0[0]), Some("a"));
        assert_eq!(get_style(&txt.0[0]), Some(&not_underlined));
        assert_eq!(get_text(&txt.0[1]), Some("_"));
        assert_eq!(get_style(&txt.0[1]), Some(&not_underlined));
    }

    #[test]
    fn test_visualize_newline() {
        let mut comp = TextInputComponent::new(
            SharedTheme::default(),
            SharedKeyConfig::default(),
            "",
            "",
        );

        let theme = SharedTheme::default();
        let _underlined = theme
            .text(false, false)
            .add_modifier(Modifier::UNDERLINED);

        comp.set_text("a\nb");
        comp.incr_cursor();

        let txt = &comp.get_draw_text();

        assert_eq!(txt.len(), 2);
        let l1_spans = &txt[0].0;
        let l2_spans = &txt[1].0;
        assert_eq!(get_text(&l1_spans[0]), Some("a"));
        assert_eq!(get_text(&l1_spans[1]), Some("_"));
        //assert_eq!(get_style(&l1_spans[1]), Some(&underlined));
        assert_eq!(get_text(&l2_spans[0]), Some("b"));
    }

    #[test]
    fn test_invisible_newline() {
        let mut comp = TextInputComponent::new(
            SharedTheme::default(),
            SharedKeyConfig::default(),
            "",
            "",
        );

        let theme = SharedTheme::default();
        let underlined = theme
            .text(true, false)
            .add_modifier(Modifier::UNDERLINED);

        comp.set_text("a\nb");

        let txt = &comp.get_draw_text();
        assert_eq!(txt.len(), 2);
        let l1_spans = &txt[0].0;
        let l2_spans = &txt[1].0;

        assert_eq!(get_text(&l1_spans[0]), Some("a"));
        assert_eq!(get_style(&l1_spans[0]), Some(&underlined));
        assert_eq!(get_text(&l2_spans[0]), Some("b"));
    }

    fn get_text<'a>(t: &'a Span) -> Option<&'a str> {
        Some(&t.content)
    }

    fn get_style<'a>(t: &'a Span) -> Option<&'a Style> {
        Some(&t.style)
    }
}
