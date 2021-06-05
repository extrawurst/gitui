use super::{
    textinput::TextInputComponent, visibility_blocking,
    CommandBlocking, CommandInfo, Component, DrawableComponent,
    EventState,
};
use crate::{
    keys::SharedKeyConfig,
    queue::{InternalEvent, NeedsUpdate, Queue},
    strings,
    tabs::StashingOptions,
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{sync, CWD};
use crossterm::event::Event;
use tui::{backend::Backend, layout::Rect, Frame};

pub struct StashMsgComponent {
    options: StashingOptions,
    input: TextInputComponent,
    queue: Queue,
    key_config: SharedKeyConfig,
}

impl DrawableComponent for StashMsgComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        self.input.draw(f, rect)?;

        Ok(())
    }
}

impl Component for StashMsgComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.is_visible() || force_all {
            self.input.commands(out, force_all);

            out.push(CommandInfo::new(
                strings::commands::stashing_confirm_msg(
                    &self.key_config,
                ),
                true,
                true,
            ));
        }

        visibility_blocking(self)
    }

    fn event(&mut self, ev: Event) -> Result<EventState> {
        if self.is_visible() {
            if self.input.event(ev)?.is_consumed() {
                return Ok(EventState::Consumed);
            }

            if let Event::Key(e) = ev {
                if e == self.key_config.enter {
                    match sync::stash_save(
                        CWD,
                        if self.input.get_text().is_empty() {
                            None
                        } else {
                            Some(self.input.get_text().as_str())
                        },
                        self.options.stash_untracked,
                        self.options.keep_index,
                    ) {
                        Ok(_) => {
                            self.input.clear();
                            self.hide();

                            self.queue.borrow_mut().push_back(
                                InternalEvent::Update(
                                    NeedsUpdate::ALL,
                                ),
                            );
                        }
                        Err(e) => {
                            self.hide();
                            log::error!(
                                "e: {} (options: {:?})",
                                e,
                                self.options
                            );
                            self.queue.borrow_mut().push_back(
                                InternalEvent::ShowErrorMsg(format!(
                                    "stash error:\n{}\noptions:\n{:?}",
                                    e, self.options
                                )),
                            );
                        }
                    }
                }

                // stop key event propagation
                return Ok(EventState::Consumed);
            }
        }
        Ok(EventState::NotConsumed)
    }

    fn is_visible(&self) -> bool {
        self.input.is_visible()
    }

    fn hide(&mut self) {
        self.input.hide();
    }

    fn show(&mut self) -> Result<()> {
        self.input.show()?;

        Ok(())
    }
}

impl StashMsgComponent {
    ///
    pub fn new(
        queue: Queue,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            options: StashingOptions::default(),
            queue,
            input: TextInputComponent::new(
                theme,
                key_config.clone(),
                &strings::stash_popup_title(&key_config),
                &strings::stash_popup_msg(&key_config),
                true,
            ),
            key_config,
        }
    }

    ///
    pub fn options(&mut self, options: StashingOptions) {
        self.options = options;
    }
}
