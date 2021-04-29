use super::{
    externaleditor::show_editor, textinput::TextInputComponent,
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent, EventState,
};
use crate::{
    app::EditorSource,
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

pub struct RewordComponent {
    input: TextInputComponent,
    commit_id: Option<CommitId>,
    queue: Queue,
    key_config: SharedKeyConfig,
}

impl DrawableComponent for RewordComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        self.input.draw(f, rect)?;

        Ok(())
    }
}

impl Component for RewordComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.is_visible() || force_all {
            self.input.commands(out, force_all);

            out.push(CommandInfo::new(
                strings::commands::reword_commit_confirm_msg(
                    &self.key_config,
                ),
                true,
                true,
            ));

            out.push(CommandInfo::new(
                strings::commands::commit_open_editor(
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
            if let Ok(EventState::Consumed) = self.input.event(ev) {
                return Ok(EventState::Consumed);
            }

            if let Event::Key(e) = ev {
                if e == self.key_config.enter {
                    self.reword()
                } else if e == self.key_config.open_commit_editor {
                    self.queue.borrow_mut().push_back(
                        InternalEvent::OpenExternalEditor(
                            None,
                            EditorSource::Reword,
                        ),
                    );
                    self.hide();
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
        self.input.hide()
    }

    fn show(&mut self) -> Result<()> {
        self.input.show()?;

        Ok(())
    }
}

impl RewordComponent {
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
                &strings::reword_popup_title(&key_config),
                &strings::reword_popup_msg(&key_config),
                true,
            ),
            commit_id: None,
            key_config,
        }
    }

    ///
    pub fn open(&mut self, id: CommitId) -> Result<()> {
        self.commit_id = Some(id);
        if let Some(commit_msg) =
            sync::get_commit_details(CWD, id)?.message
        {
            self.input.set_text(commit_msg.combine());
        }
        self.show()?;

        Ok(())
    }

    /// Open external editor
    pub fn show_editor(&mut self) -> Result<()> {
        let message = show_editor(Some(self.input.get_text()))?
            .trim()
            .to_string();

        self.input.set_text(message);
        self.input.show()?;

        Ok(())
    }

    ///
    pub fn reword(&mut self) {
        if let Some(commit_id) = self.commit_id {
            match sync::reword(
                CWD,
                commit_id.into(),
                self.input.get_text(),
            ) {
                Ok(_) => {
                    self.input.clear();
                    self.hide();

                    self.queue.borrow_mut().push_back(
                        InternalEvent::Update(NeedsUpdate::ALL),
                    );
                }
                Err(e) => {
                    self.input.clear();
                    self.hide();
                    log::error!("e: {}", e,);
                    self.queue.borrow_mut().push_back(
                        InternalEvent::ShowErrorMsg(format!(
                            "reword error:\n{}",
                            e,
                        )),
                    );
                }
            }
        }
    }
}
