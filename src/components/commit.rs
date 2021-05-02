use super::{
    textinput::TextInputComponent, visibility_blocking,
    CommandBlocking, CommandInfo, Component, DrawableComponent,
    EventState, ExternalEditorComponent,
};
use crate::{
    get_app_config_path,
    keys::SharedKeyConfig,
    queue::{InternalEvent, NeedsUpdate, Queue},
    strings,
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
    cached,
    sync::{self, utils::get_config_string, CommitId, HookResult},
    CWD,
};
use crossterm::event::Event;
use easy_cast::Cast;
use std::{
    fs::{read_to_string, File},
    io::{Read, Write},
    path::PathBuf,
};
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    widgets::Paragraph,
    Frame,
};

pub struct CommitComponent {
    input: TextInputComponent,
    amend: Option<CommitId>,
    queue: Queue,
    key_config: SharedKeyConfig,
    git_branch_name: cached::BranchName,
    commit_template: Option<String>,
    theme: SharedTheme,
}

const FIRST_LINE_LIMIT: usize = 50;

impl DrawableComponent for CommitComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        if self.is_visible() {
            self.input.draw(f, rect)?;
            self.draw_branch_name(f);
            self.draw_warnings(f);
        }

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

    fn event(&mut self, ev: Event) -> Result<EventState> {
        if self.is_visible() {
            if self.input.event(ev)?.is_consumed() {
                return Ok(EventState::Consumed);
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
                        InternalEvent::OpenExternalEditor(None),
                    );
                    self.hide();
                } else {
                }
                // stop key event propagation
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
        if self.amend.is_some() {
            self.input.clear();
        }
        self.amend = None;

        self.input
            .set_title(strings::commit_title(&self.key_config));

        self.commit_template =
            get_config_string(CWD, "commit.template")
                .ok()
                .flatten()
                .and_then(|path| read_to_string(path).ok());

        if self.is_empty() {
            if let Some(s) = &self.commit_template {
                self.input.set_text(s.clone());
            }
        }

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
                theme.clone(),
                key_config.clone(),
                "",
                &strings::commit_msg(&key_config),
                true,
            ),
            key_config,
            git_branch_name: cached::BranchName::new(CWD),
            commit_template: None,
            theme,
        }
    }

    ///
    pub fn update(&mut self) -> Result<()> {
        self.git_branch_name.lookup().map(Some).unwrap_or(None);

        Ok(())
    }

    fn draw_branch_name<B: Backend>(&self, f: &mut Frame<B>) {
        if let Some(name) = self.git_branch_name.last() {
            let w = Paragraph::new(format!("{{{}}}", name))
                .alignment(Alignment::Right);

            let rect = {
                let mut rect = self.input.get_area();
                rect.height = 1;
                rect.width = rect.width.saturating_sub(1);
                rect
            };

            f.render_widget(w, rect);
        }
    }

    fn draw_warnings<B: Backend>(&self, f: &mut Frame<B>) {
        let first_line = self
            .input
            .get_text()
            .lines()
            .next()
            .map(str::len)
            .unwrap_or_default();

        if first_line > FIRST_LINE_LIMIT {
            let msg = strings::commit_first_line_warning(first_line);
            let msg_length: u16 = msg.len().cast();
            let w =
                Paragraph::new(msg).style(self.theme.text_danger());

            let rect = {
                let mut rect = self.input.get_area();
                rect.y += rect.height.saturating_sub(1);
                rect.height = 1;
                let offset =
                    rect.width.saturating_sub(msg_length + 1);
                rect.width = rect.width.saturating_sub(offset + 1);
                rect.x += offset;

                rect
            };

            f.render_widget(w, rect);
        }
    }

    pub fn show_editor(&mut self) -> Result<()> {
        const COMMIT_MSG_FILE_NAME: &str = "COMMITMSG_EDITOR";
        //TODO: use a tmpfile here
        let mut config_path: PathBuf = get_app_config_path()?;
        config_path.push(COMMIT_MSG_FILE_NAME);

        {
            let mut file = File::create(&config_path)?;
            file.write_fmt(format_args!(
                "{}\n",
                self.input.get_text()
            ))?;
            file.write_all(
                strings::commit_editor_msg(&self.key_config)
                    .as_bytes(),
            )?;
        }

        ExternalEditorComponent::open_file_in_editor(&config_path)?;

        let mut message = String::new();

        let mut file = File::open(&config_path)?;
        file.read_to_string(&mut message)?;
        drop(file);
        std::fs::remove_file(&config_path)?;

        let message: String = message
            .lines()
            .flat_map(|l| {
                if l.starts_with('#') {
                    vec![]
                } else {
                    vec![l, "\n"]
                }
            })
            .collect();

        let message = message.trim().to_string();

        self.input.set_text(message);
        self.input.show()?;

        Ok(())
    }

    fn commit(&mut self) -> Result<()> {
        let msg = self.input.get_text().clone();
        self.input.clear();
        self.commit_msg(msg)
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
        !self.is_empty() && self.is_changed()
    }

    fn can_amend(&self) -> bool {
        self.amend.is_none()
            && sync::get_head(CWD).is_ok()
            && (self.is_empty() || !self.is_changed())
    }

    fn is_empty(&self) -> bool {
        self.input.get_text().is_empty()
    }

    fn is_changed(&self) -> bool {
        Some(self.input.get_text().trim())
            != self.commit_template.as_ref().map(|s| s.trim())
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
