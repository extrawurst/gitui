use super::{
    textinput::TextInputComponent, CommandBlocking, CommandInfo,
    Component, DrawableComponent,
};
use crate::{
    keys::SharedKeyConfig,
    queue::{InternalEvent, Queue},
    strings,
    ui::style::SharedTheme,
};
use anyhow::Result;
use crossterm::event::Event;
use tui::{backend::Backend, layout::Rect, Frame};

pub struct FindCommitComponent {
    input: TextInputComponent,
    queue: Queue,
    is_focused: bool,
    visible: bool,
    key_config: SharedKeyConfig,
}

impl DrawableComponent for FindCommitComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        self.input.draw(f, rect)?;
        Ok(())
    }
}

impl Component for FindCommitComponent {
    fn commands(
        &self,
        _out: &mut Vec<CommandInfo>,
        _force_all: bool,
    ) -> CommandBlocking {
        CommandBlocking::PassingOn
    }

    fn event(&mut self, ev: Event) -> Result<bool> {
        if self.is_visible() && self.focused() {
            if let Event::Key(e) = ev {
                if e == self.key_config.exit_popup {
                    // Prevent text input closing
                    self.focus(false);
                    return Ok(true);
                }
            }
            if self.input.event(ev)? {
                self.queue.borrow_mut().push_back(
                    InternalEvent::FilterLog(
                        self.input.get_text().to_string(),
                    ),
                );
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn is_visible(&self) -> bool {
        return self.visible;
    }

    fn hide(&mut self) {
        self.visible = false;
    }
    fn show(&mut self) -> Result<()> {
        self.visible = true;
        Ok(())
    }

    fn focus(&mut self, focus: bool) {
        self.is_focused = focus;
    }

    fn focused(&self) -> bool {
        return self.is_focused;
    }

    fn toggle_visible(&mut self) -> Result<()> {
        self.visible = !self.visible;
        Ok(())
    }
}

impl FindCommitComponent {
    ///
    pub fn new(
        queue: Queue,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        let mut input_component = TextInputComponent::new(
            theme,
            key_config.clone(),
            &strings::find_commit_title(&key_config),
            &strings::find_commit_msg(&key_config),
            true,
        );
        input_component.show().expect("Will not error");
        input_component.set_should_use_rect(true);
        Self {
            queue,
            input: input_component,
            key_config,
            visible: false,
            is_focused: false,
        }
    }
}
