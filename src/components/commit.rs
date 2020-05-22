use super::{
    textinput::TextInputComponent, visibility_blocking,
    CommandBlocking, CommandInfo, Component, DrawableComponent,
};
use crate::{
    queue::{InternalEvent, NeedsUpdate, Queue},
    strings,
    ui::style::Theme,
};
use asyncgit::{sync, CWD};
use crossterm::event::{Event, KeyCode};
use log::error;
use strings::commands;
use sync::HookResult;
use tui::{backend::Backend, layout::Rect, Frame};

pub struct CommitComponent {
    input: TextInputComponent,
    queue: Queue,
}

impl DrawableComponent for CommitComponent {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, rect: Rect) {
        self.input.draw(f, rect)
    }
}

impl Component for CommitComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        _force_all: bool,
    ) -> CommandBlocking {
        out.push(CommandInfo::new(
            commands::COMMIT_ENTER,
            self.can_commit(),
            self.is_visible(),
        ));
        out.push(CommandInfo::new(
            commands::CLOSE_POPUP,
            true,
            self.is_visible(),
        ));
        visibility_blocking(self)
    }

    fn event(&mut self, ev: Event) -> bool {
        if self.is_visible() {
            if self.input.event(ev) {
                return true;
            }

            if let Event::Key(e) = ev {
                match e.code {
                    KeyCode::Enter if self.can_commit() => {
                        self.commit();
                    }
                    _ => (),
                };

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

impl CommitComponent {
    ///
    pub fn new(queue: Queue, theme: &Theme) -> Self {
        Self {
            queue,
            input: TextInputComponent::new(theme),
        }
    }

    fn commit(&mut self) {
        let mut msg = self.input.get_text().clone();
        if let HookResult::NotOk(e) =
            sync::hooks_commit_msg(CWD, &mut msg).unwrap()
        {
            error!("commit-msg hook error: {}", e);
            self.queue.borrow_mut().push_back(
                InternalEvent::ShowErrorMsg(format!(
                    "commit-msg hook error:\n{}",
                    e
                )),
            );
            return;
        }

        if let Err(e) = sync::commit(CWD, &msg) {
            error!("commit error: {}", &e);
            self.queue.borrow_mut().push_back(
                InternalEvent::ShowErrorMsg(format!(
                    "commit failed:\n{}",
                    &e
                )),
            );
            return;
        }

        if let HookResult::NotOk(e) =
            sync::hooks_post_commit(CWD).unwrap()
        {
            error!("post-commit hook error: {}", e);
            self.queue.borrow_mut().push_back(
                InternalEvent::ShowErrorMsg(format!(
                    "post-commit hook error:\n{}",
                    e
                )),
            );
        }

        self.input.clear();
        self.hide();

        self.queue
            .borrow_mut()
            .push_back(InternalEvent::Update(NeedsUpdate::ALL));
    }

    fn can_commit(&self) -> bool {
        !self.input.get_text().is_empty()
    }
}
