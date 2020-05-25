use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent,
};
use crate::{
    components::dialog_paragraph,
    queue::{Action, InternalEvent, Queue},
    strings, ui,
    ui::style::Theme,
};
use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use std::borrow::Cow;
use strings::commands;
use tui::{
    backend::Backend,
    layout::Rect,
    widgets::{Clear, Text},
    Frame,
};

///
pub struct ResetComponent {
    target: Option<Action>,
    visible: bool,
    queue: Queue,
    theme: Theme,
}

impl DrawableComponent for ResetComponent {
    fn draw<B: Backend>(
        &mut self,
        f: &mut Frame<B>,
        _rect: Rect,
    ) -> Result<()> {
        if self.visible {
            let (title, msg) = self.get_text();

            let txt = vec![Text::Styled(
                Cow::from(msg),
                self.theme.text_danger(),
            )];

            let area = ui::centered_rect(30, 20, f.size());
            f.render_widget(Clear, area);
            f.render_widget(
                dialog_paragraph(title, txt.iter()),
                area,
            );
        }

        Ok(())
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

    fn event(&mut self, ev: Event) -> Result<bool> {
        if self.visible {
            if let Event::Key(e) = ev {
                return match e.code {
                    KeyCode::Esc => {
                        self.hide();
                        Ok(true)
                    }

                    KeyCode::Enter => {
                        self.confirm();
                        Ok(true)
                    }

                    _ => Ok(true),
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

impl ResetComponent {
    ///
    pub fn new(queue: Queue, theme: &Theme) -> Self {
        Self {
            target: None,
            visible: false,
            queue,
            theme: *theme,
        }
    }
    ///
    pub fn open(&mut self, a: Action) -> Result<()> {
        self.target = Some(a);
        self.show()?;

        Ok(())
    }
    ///
    pub fn confirm(&mut self) {
        if let Some(a) = self.target.take() {
            self.queue
                .borrow_mut()
                .push_back(InternalEvent::ConfirmedAction(a));
        }

        self.hide();
    }

    fn get_text(&self) -> (&str, &str) {
        if let Some(ref a) = self.target {
            return match a {
                Action::Reset(_) => (
                    strings::CONFIRM_TITLE_RESET,
                    strings::CONFIRM_MSG_RESET,
                ),
                Action::StashDrop(_) => (
                    strings::CONFIRM_TITLE_STASHDROP,
                    strings::CONFIRM_MSG_STASHDROP,
                ),
            };
        }

        ("", "")
    }
}
