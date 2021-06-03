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
use easy_cast::Cast;
use tui::{
    backend::Backend, layout::Rect, widgets::Paragraph, Frame,
};

pub struct CreateBranchComponent {
    input: TextInputComponent,
    queue: Queue,
    key_config: SharedKeyConfig,
    theme: SharedTheme,
    invalid_name: bool,
    already_exists: bool,
}

impl DrawableComponent for CreateBranchComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        self.input.draw(f, rect)?;
        self.draw_warnings(f);

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
                self.validate_input()?;
                if e == self.key_config.enter {
                    self.create_branch()
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
                theme.clone(),
                key_config.clone(),
                &strings::create_branch_popup_title(&key_config),
                &strings::create_branch_popup_msg(&key_config),
                true,
            ),
            key_config,
            theme,
            invalid_name: false,
            already_exists: false,
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

    fn validate_input(&mut self) -> Result<()> {
        let branch_name = self.input.get_text().as_str();

        self.already_exists = false;
        self.invalid_name = false;

        let branches = sync::get_branches_info(CWD, true)?;
        for branch in &branches {
            if branch.name == self.input.get_text().as_str() {
                self.already_exists = true;
            }
        }
        self.invalid_name = sync::branch_name_is_valid(branch_name)?;

        Ok(())
    }

    fn draw_warnings<B: Backend>(&self, f: &mut Frame<B>) {
        let branch_name = self.input.get_text().as_str();

        let msg;
        if branch_name.is_empty() {
            return;
        } else if self.invalid_name {
            msg = strings::branch_invalid_name_warning();
        } else if self.already_exists {
            msg = strings::branch_already_exists();
        } else {
            return;
        }

        let msg_length: u16 = msg.len().cast();
        let w = Paragraph::new(msg).style(self.theme.text_danger());

        let rect = {
            let mut rect = self.input.get_area();
            rect.y += rect.height.saturating_sub(1);
            rect.height = 1;
            let offset = rect.width.saturating_sub(msg_length + 1);
            rect.width = rect.width.saturating_sub(offset + 1);
            rect.x += offset;

            rect
        };

        f.render_widget(w, rect);
    }
}
