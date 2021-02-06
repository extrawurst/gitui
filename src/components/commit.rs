use super::{
    externaleditor::show_editor, textinput::TextInputComponent,
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent,
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
    sync::{self, CommitId, HookResult},
    CWD,
};
use crossterm::event::Event;
use tui::{backend::Backend, layout::Rect, Frame};

pub struct CommitComponent {
    input: TextInputComponent,
    amend: Option<CommitId>,
    queue: Queue,
    key_config: SharedKeyConfig,
}

impl DrawableComponent for CommitComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        self.input.draw(f, rect)?;

        Ok(())
    }
}

impl Component for CommitComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        self.input.commands(out, force_all);

        if self.is_visible() || force_all {
            out.push(CommandInfo::new(
                strings::commands::commit_enter(&self.key_config),
                self.can_commit(),
                true,
            ));

            out.push(CommandInfo::new(
                strings::commands::commit_amend(&self.key_config),
                self.can_amend(),
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

    fn event(&mut self, ev: Event) -> Result<bool> {
        if self.is_visible() {
            if self.input.event(ev)? {
                return Ok(true);
            }

            if let Event::Key(e) = ev {
                if e == self.key_config.enter && self.can_commit() {
                    self.commit()?;
                } else if e == self.key_config.commit_amend
                    && self.can_amend()
                {
                    self.amend()?;
                } else if e == self.key_config.open_commit_editor {
                    self.queue.borrow_mut().push_back(
                        InternalEvent::OpenExternalEditor(
                            None,
                            EditorSource::Commit,
                        ),
                    );
                    self.hide();
                } else {
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
        self.amend = None;

        self.input.clear();
        self.input
            .set_title(strings::commit_title(&self.key_config));
        self.input.show()?;

        Ok(())
    }
}

impl CommitComponent {
    ///
    pub fn new(
        queue: Queue,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            queue,
            amend: None,
            input: TextInputComponent::new(
                theme,
                key_config.clone(),
                "",
                &strings::commit_msg(&key_config),
                true,
            ),
            key_config,
        }
    }

    pub fn show_editor(&mut self) -> Result<()> {
        let message = show_editor(Some(self.input.get_text()))?
            .trim()
            .to_string();

        self.input.set_text(message);
        self.input.show()?;

        Ok(())
    }

    fn commit(&mut self) -> Result<()> {
        self.commit_msg(self.input.get_text().clone())
    }

    fn commit_msg(&mut self, msg: String) -> Result<()> {
        if let HookResult::NotOk(e) = sync::hooks_pre_commit(CWD)? {
            log::error!("pre-commit hook error: {}", e);
            self.queue.borrow_mut().push_back(
                InternalEvent::ShowErrorMsg(format!(
                    "pre-commit hook error:\n{}",
                    e
                )),
            );
            return Ok(());
        }
        let mut msg = msg;
        if let HookResult::NotOk(e) =
            sync::hooks_commit_msg(CWD, &mut msg)?
        {
            log::error!("commit-msg hook error: {}", e);
            self.queue.borrow_mut().push_back(
                InternalEvent::ShowErrorMsg(format!(
                    "commit-msg hook error:\n{}",
                    e
                )),
            );
            return Ok(());
        }

        let res = self.amend.map_or_else(
            || sync::commit(CWD, &msg),
            |amend| sync::amend(CWD, amend, &msg),
        );
        if let Err(e) = res {
            log::error!("commit error: {}", &e);
            self.queue.borrow_mut().push_back(
                InternalEvent::ShowErrorMsg(format!(
                    "commit failed:\n{}",
                    &e
                )),
            );
            return Ok(());
        }

        if let HookResult::NotOk(e) = sync::hooks_post_commit(CWD)? {
            log::error!("post-commit hook error: {}", e);
            self.queue.borrow_mut().push_back(
                InternalEvent::ShowErrorMsg(format!(
                    "post-commit hook error:\n{}",
                    e
                )),
            );
        }

        self.hide();

        self.queue
            .borrow_mut()
            .push_back(InternalEvent::Update(NeedsUpdate::ALL));

        Ok(())
    }

    fn can_commit(&self) -> bool {
        !self.input.get_text().is_empty()
    }

    fn can_amend(&self) -> bool {
        self.amend.is_none()
            && sync::get_head(CWD).is_ok()
            && self.input.get_text().is_empty()
    }

    fn amend(&mut self) -> Result<()> {
        let id = sync::get_head(CWD)?;
        self.amend = Some(id);

        let details = sync::get_commit_details(CWD, id)?;

        self.input
            .set_title(strings::commit_title_amend(&self.key_config));

        if let Some(msg) = details.message {
            self.input.set_text(msg.combine());
        }

        Ok(())
    }
}
