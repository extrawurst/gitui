use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent,
};
use crate::{
    queue::{InternalEvent, Queue, ResetItem},
    strings, ui,
};

use crate::components::dialog_paragraph;
use crate::ui::style::Theme;
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
    target: Option<ResetItem>,
    visible: bool,
    queue: Queue,
    theme: Theme,
}

impl DrawableComponent for ResetComponent {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, _rect: Rect) {
        if self.visible {
            let mut txt = Vec::new();
            txt.push(Text::Styled(
                Cow::from(strings::RESET_MSG),
                self.theme.text_danger(),
            ));

            let area = ui::centered_rect(30, 20, f.size());
            f.render_widget(Clear, area);
            f.render_widget(
                dialog_paragraph(strings::RESET_TITLE, txt.iter()),
                area,
            );
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

    fn event(&mut self, ev: Event) -> bool {
        if self.visible {
            if let Event::Key(e) = ev {
                return match e.code {
                    KeyCode::Esc => {
                        self.hide();
                        true
                    }

                    KeyCode::Enter => {
                        self.confirm();
                        true
                    }

                    _ => true,
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
    pub fn open_for_path(&mut self, item: ResetItem) {
        self.target = Some(item);
        self.show();
    }
    ///
    pub fn confirm(&mut self) {
        if let Some(target) = self.target.take() {
            self.queue
                .borrow_mut()
                .push_back(InternalEvent::ResetItem(target));
        }

        self.hide();
    }
}
