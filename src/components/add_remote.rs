use super::{
    textinput::TextInputComponent, visibility_blocking,
    CommandBlocking, CommandInfo, Component, DrawableComponent,
};
use crate::{
    keys::SharedKeyConfig,
    queue::{InternalEvent, Queue},
    strings,
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
    sync::{self},
    CWD,
};
use crossterm::event::Event;
use tui::layout::Rect;
use tui::{backend::Backend, Frame};

pub struct AddRemoteComponent {
    input_name: TextInputComponent,
    input_url: TextInputComponent,
    input_focused: usize,
    queue: Queue,
    key_config: SharedKeyConfig,
}

impl DrawableComponent for AddRemoteComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        if self.input_focused == 0 {
            self.input_name.draw(f, rect)?;
        } else {
            self.input_url.draw(f, rect)?;
        }

        Ok(())
    }
}

impl Component for AddRemoteComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.is_visible() || force_all {
            self.input_name.commands(out, force_all);

            out.push(CommandInfo::new(
                strings::commands::add_remote_confirm_msg(
                    &self.key_config,
                ),
                true,
                true,
            ));
        }

        visibility_blocking(self)
    }

    fn event(&mut self, ev: Event) -> Result<bool> {
        if self.is_visible() {
            if (self.input_focused == 0
                && self.input_name.event(ev)?)
                || (self.input_focused == 1
                    && self.input_url.event(ev)?)
            {
                return Ok(true);
            }

            if let Event::Key(e) = ev {
                if e == self.key_config.enter {
                    if self.input_focused == 0 {
                        self.input_focused = 1;
                        self.input_name.hide();
                        self.input_url.show()?;
                    } else {
                        self.input_focused = 0;
                        self.add_remote();
                    }
                } else if e == self.key_config.move_up {
                    self.input_focused = 0;
                } else if e == self.key_config.move_down {
                    self.input_focused = 1;
                } else if e == self.key_config.exit_popup {
                    self.hide();
                }

                return Ok(true);
            }
        }
        Ok(false)
    }

    fn is_visible(&self) -> bool {
        self.input_name.is_visible() || self.input_url.is_visible()
    }

    fn hide(&mut self) {
        self.input_name.hide();
        self.input_url.hide();
    }

    fn show(&mut self) -> Result<()> {
        self.input_name.show()?;

        Ok(())
    }
}

impl AddRemoteComponent {
    ///
    pub fn new(
        queue: Queue,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            queue,
            input_name: TextInputComponent::new(
                theme.clone(),
                key_config.clone(),
                strings::ADD_REMOTE_POPUP_TITLE,
                strings::ADD_REMOTE_POPUP_MSG_NAME,
            ),
            input_url: TextInputComponent::new(
                theme,
                key_config.clone(),
                strings::ADD_REMOTE_POPUP_TITLE,
                strings::ADD_REMOTE_POPUP_MSG_URL,
            ),
            input_focused: 0,
            key_config,
        }
    }

    ///
    pub fn open(&mut self) -> Result<()> {
        self.show()?;

        Ok(())
    }

    ///
    pub fn add_remote(&mut self) {
        let res = sync::add_remote(
            CWD,
            self.input_name.get_text().as_str(),
            self.input_url.get_text().as_str(),
        );

        self.input_name.clear();
        self.hide();

        match res {
            Ok(_) => {}
            Err(e) => {
                log::error!("add remote: {}", e,);
                self.queue.borrow_mut().push_back(
                    InternalEvent::ShowErrorMsg(format!(
                        "add remote error:\n{}",
                        e,
                    )),
                );
            }
        }
    }
}
