use super::{
    textinput::TextInputComponent, visibility_blocking,
    CommandBlocking, CommandInfo, Component, DrawableComponent,
    ExternalEditorComponent,
};
use crate::{
    app::EditorSource,
    get_app_config_path,
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
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};
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

    fn event(&mut self, ev: Event) -> Result<bool> {
        if self.is_visible() {
            if self.input.event(ev)? {
                return Ok(true);
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

    /// After an external editor has been open,
    /// this should be called to put the text in the
    /// right place
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
