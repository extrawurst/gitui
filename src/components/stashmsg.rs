use super::{
    textinput::TextInputComponent, visibility_blocking,
    CommandBlocking, CommandInfo, Component, DrawableComponent,
};
use crate::{
    queue::{InternalEvent, NeedsUpdate, Queue},
    strings,
    tabs::StashingOptions,
    ui::style::Theme,
};
use anyhow::Result;
use asyncgit::{sync, CWD};
use crossterm::event::{Event, KeyCode};
use strings::commands;
use tui::{backend::Backend, layout::Rect, Frame};

pub struct StashMsgComponent {
    options: StashingOptions,
    input: TextInputComponent,
    queue: Queue,
}

impl DrawableComponent for StashMsgComponent {
    fn draw<B: Backend>(
        &mut self,
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
        self.input.commands(out, force_all);

        out.push(CommandInfo::new(
            commands::STASHING_CONFIRM_MSG,
            true,
            self.is_visible() || force_all,
        ));
        visibility_blocking(self)
    }

    fn event(&mut self, ev: Event) -> Result<bool> {
        if self.is_visible() {
            if self.input.event(ev)? {
                return Ok(true);
            }

            if let Event::Key(e) = ev {
                if let KeyCode::Enter = e.code {
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
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn is_visible(&self) -> bool {
        self.input.is_visible()
    }

    fn hide(&mut self) {
        self.input.hide()
    }

    fn show(&mut self) -> Result<()> {
        self.input.show()?;

        Ok(())
    }
}

impl StashMsgComponent {
    ///
    pub fn new(queue: Queue, theme: &Theme) -> Self {
        Self {
            options: StashingOptions::default(),
            queue,
            input: TextInputComponent::new(
                theme,
                strings::STASH_POPUP_TITLE,
                strings::STASH_POPUP_MSG,
            ),
        }
    }

    ///
    pub fn options(&mut self, options: StashingOptions) {
        self.options = options;
    }
}
