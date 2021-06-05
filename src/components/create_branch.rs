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
use asyncgit::{sync, CWD};
use crossterm::event::Event;
use tui::{backend::Backend, layout::Rect, Frame};

pub struct CreateBranchComponent {
    input: TextInputComponent,
    queue: Queue,
    key_config: SharedKeyConfig,
}

impl DrawableComponent for CreateBranchComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        self.input.draw(f, rect)?;

        Ok(())
    }
}

impl Component for CreateBranchComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.is_visible() || force_all {
            self.input.commands(out, force_all);

            out.push(CommandInfo::new(
                strings::commands::create_branch_confirm_msg(
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
                    self.create_branch();
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

impl CreateBranchComponent {
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
                &strings::create_branch_popup_title(&key_config),
                &strings::create_branch_popup_msg(&key_config),
                true,
            ),
            key_config,
        }
    }

    ///
    pub fn open(&mut self) -> Result<()> {
        self.show()?;

        Ok(())
    }

    ///
    pub fn create_branch(&mut self) {
        let res =
            sync::create_branch(CWD, self.input.get_text().as_str());

        self.input.clear();
        self.hide();

        match res {
            Ok(_) => {
                self.queue.borrow_mut().push_back(
                    InternalEvent::Update(NeedsUpdate::BRANCHES),
                );
            }
            Err(e) => {
                log::error!("create branch: {}", e,);
                self.queue.borrow_mut().push_back(
                    InternalEvent::ShowErrorMsg(format!(
                        "create branch error:\n{}",
                        e,
                    )),
                );
            }
        }
    }
}
