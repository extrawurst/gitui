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
    sync::{self},
    CWD,
};
use crossterm::event::Event;
use tui::{backend::Backend, layout::Rect, Frame};

pub struct RenameBranchComponent {
    input: TextInputComponent,
    branch_ref: Option<String>,
    queue: Queue,
    key_config: SharedKeyConfig,
}

impl DrawableComponent for RenameBranchComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        self.input.draw(f, rect)?;

        Ok(())
    }
}

impl Component for RenameBranchComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.is_visible() || force_all {
            self.input.commands(out, force_all);

            out.push(CommandInfo::new(
                strings::commands::rename_branch_confirm_msg(
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
                    self.rename_branch();
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

impl RenameBranchComponent {
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
                &strings::rename_branch_popup_title(&key_config),
                &strings::rename_branch_popup_msg(&key_config),
                true,
            ),
            branch_ref: None,
            key_config,
        }
    }

    ///
    pub fn open(
        &mut self,
        branch_ref: String,
        cur_name: String,
    ) -> Result<()> {
        self.branch_ref = None;
        self.branch_ref = Some(branch_ref);
        self.input.set_text(cur_name);
        self.show()?;

        Ok(())
    }

    ///
    pub fn rename_branch(&mut self) {
        if let Some(br) = &self.branch_ref {
            let res = sync::rename_branch(
                CWD,
                br,
                self.input.get_text().as_str(),
            );

            match res {
                Ok(_) => {
                    self.queue.borrow_mut().push_back(
                        InternalEvent::Update(NeedsUpdate::ALL),
                    );
                    self.hide();
                    self.queue
                        .borrow_mut()
                        .push_back(InternalEvent::SelectBranch);
                }
                Err(e) => {
                    log::error!("create branch: {}", e,);
                    self.queue.borrow_mut().push_back(
                        InternalEvent::ShowErrorMsg(format!(
                            "rename branch error:\n{}",
                            e,
                        )),
                    );
                }
            }
        } else {
            log::error!("create branch: No branch selected");
            self.queue
                .borrow_mut()
                .push_back(InternalEvent::ShowErrorMsg(
                "rename branch error: No branch selected to rename"
                    .to_string(),
            ));
        }

        self.input.clear();
    }
}
