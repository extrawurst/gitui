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
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, rect: Rect) {
        self.input.draw(f, rect)
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

    fn event(&mut self, ev: Event) -> bool {
        if self.is_visible() {
            if self.input.event(ev) {
                return true;
            }

            if let Event::Key(e) = ev {
                if let KeyCode::Enter = e.code {
                    if sync::stash_save(
                        CWD,
                        if self.input.get_text().is_empty() {
                            None
                        } else {
                            Some(self.input.get_text().as_str())
                        },
                        self.options.stash_untracked,
                        self.options.keep_index,
                    )
                    .is_ok()
                    {
                        self.input.clear();
                        self.hide();

                        self.queue.borrow_mut().push_back(
                            InternalEvent::Update(NeedsUpdate::ALL),
                        );
                    }
                }

                // stop key event propagation
                return true;
            }
        }
        false
    }

    fn is_visible(&self) -> bool {
        self.input.is_visible()
    }

    fn hide(&mut self) {
        self.input.hide()
    }

    fn show(&mut self) {
        self.input.show()
    }
}

impl StashMsgComponent {
    ///
    pub fn new(queue: Queue, theme: &Theme) -> Self {
        Self {
            options: StashingOptions::default(),
            queue,
            input: TextInputComponent::new(theme),
        }
    }
}
