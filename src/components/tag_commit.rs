use super::{
    textinput::TextInputComponent, visibility_blocking,
    CommandBlocking, CommandInfo, Component, DrawableComponent,
    EventState,
};
use crate::{
    keys::SharedKeyConfig,
    queue::{InternalEvent, NeedsUpdate, Queue},
    strings,
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
    sync::{self, CommitId},
    CWD,
};
use crossterm::event::Event;
use tui::{backend::Backend, layout::Rect, Frame};

pub struct TagCommitComponent {
    input: TextInputComponent,
    commit_id: Option<CommitId>,
    queue: Queue,
    key_config: SharedKeyConfig,
}

impl DrawableComponent for TagCommitComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        self.input.draw(f, rect)?;

        Ok(())
    }
}

impl Component for TagCommitComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.is_visible() || force_all {
            self.input.commands(out, force_all);

            out.push(CommandInfo::new(
                strings::commands::tag_commit_confirm_msg(
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
                    self.tag();
                }

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

impl TagCommitComponent {
    ///
    pub fn new(
        queue: Queue,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            queue,
            input: TextInputComponent::new(
                theme,
                key_config.clone(),
                &strings::tag_commit_popup_title(&key_config),
                &strings::tag_commit_popup_msg(&key_config),
                true,
            ),
            commit_id: None,
            key_config,
        }
    }

    ///
    pub fn open(&mut self, id: CommitId) -> Result<()> {
        self.commit_id = Some(id);
        self.show()?;

        Ok(())
    }

    ///
    pub fn tag(&mut self) {
        if let Some(commit_id) = self.commit_id {
            match sync::tag(CWD, &commit_id, self.input.get_text()) {
                Ok(_) => {
                    self.input.clear();
                    self.hide();

                    self.queue.borrow_mut().push_back(
                        InternalEvent::Update(NeedsUpdate::ALL),
                    );
                }
                Err(e) => {
                    self.hide();
                    log::error!("e: {}", e,);
                    self.queue.borrow_mut().push_back(
                        InternalEvent::ShowErrorMsg(format!(
                            "tag error:\n{}",
                            e,
                        )),
                    );
                }
            }
        }
    }
}
