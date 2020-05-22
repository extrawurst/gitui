use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent,
};
use crate::{
    components::dialog_paragraph, strings, ui, ui::style::Theme,
};
use crossterm::event::{Event, KeyCode};
use std::borrow::Cow;
use strings::commands;
use tui::{
    backend::Backend,
    layout::Rect,
    style::Style,
    widgets::{Clear, Text},
    Frame,
};

/// primarily a subcomponet for user input of text (used in `CommitComponent`)
pub struct TextInputComponent {
    msg: String,
    visible: bool,
    theme: Theme,
}

impl TextInputComponent {
    ///
    pub fn new(theme: &Theme) -> Self {
        Self {
            msg: String::default(),
            visible: false,
            theme: *theme,
        }
    }

    ///
    pub fn clear(&mut self) {
        self.msg.clear();
    }

    ///
    pub fn get_text(&self) -> &String {
        &self.msg
    }
}

impl DrawableComponent for TextInputComponent {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, _rect: Rect) {
        if self.visible {
            let txt = if self.msg.is_empty() {
                [Text::Styled(
                    Cow::from(strings::COMMIT_MSG),
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
                dialog_paragraph(strings::COMMIT_TITLE, txt.iter()),
                area,
            );
        }
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

    fn event(&mut self, ev: Event) -> bool {
        if self.visible {
            if let Event::Key(e) = ev {
                match e.code {
                    KeyCode::Esc => {
                        self.hide();
                        return true;
                    }
                    KeyCode::Char(c) => {
                        self.msg.push(c);
                        return true;
                    }
                    KeyCode::Backspace => {
                        self.msg.pop();
                        return true;
                    }
                    _ => (),
                };
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
