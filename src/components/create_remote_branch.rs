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
use crossterm::event::Event;
use tui::{backend::Backend, layout::Rect, Frame};

pub struct CreateRemoteBranchComponent {
    input: TextInputComponent,
    local_branch_ref: Option<String>,
    queue: Queue,
    key_config: SharedKeyConfig,
}

impl DrawableComponent for CreateRemoteBranchComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        self.input.draw(f, rect)?;

        Ok(())
    }
}

impl Component for CreateRemoteBranchComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.is_visible() || force_all {
            self.input.commands(out, force_all);

            out.push(CommandInfo::new(
                strings::commands::create_remote_branch_confirm_msg(
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
                if e == self.key_config.enter {
                    self.create_remote_branch();
                }

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

impl CreateRemoteBranchComponent {
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
                &strings::create_remote_branch_popup_title(
                    &key_config,
                ),
                &strings::create_remote_branch_popup_msg(&key_config),
            ),
            local_branch_ref: None,
            key_config,
        }
    }

    ///
    pub fn open(&mut self, local_branch_ref: String) -> Result<()> {
        self.local_branch_ref = Some(local_branch_ref);
        self.show()?;

        Ok(())
    }

    /// This only creates a remote branch in memory
    pub fn create_remote_branch(&mut self) {
        self.queue.borrow_mut().push_back(
            InternalEvent::AddUpstreamBranch(
                self.input.get_text().clone(),
            ),
        );
        self.input.clear();
        self.hide();
        if let Some(local_branch_ref) = &self.local_branch_ref {
            self.queue.borrow_mut().push_back(
                InternalEvent::OpenUpstreamBranchPopup(
                    local_branch_ref.clone(),
                ),
            );
        }
    }
}
