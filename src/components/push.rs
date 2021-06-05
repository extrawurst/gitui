use crate::{
    components::{
        cred::CredComponent, visibility_blocking, CommandBlocking,
        CommandInfo, Component, DrawableComponent, EventState,
    },
    keys::SharedKeyConfig,
    queue::{InternalEvent, Queue},
    strings,
    ui::{self, style::SharedTheme},
};
use anyhow::Result;
use asyncgit::{
    sync::{
        cred::{
            extract_username_password, need_username_password,
            BasicAuthCredential,
        },
        get_branch_remote, get_default_remote,
    },
    AsyncNotification, AsyncPush, PushRequest, RemoteProgress,
    RemoteProgressState, CWD,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use tui::{
    backend::Backend,
    layout::Rect,
    text::Span,
    widgets::{Block, BorderType, Borders, Clear, Gauge},
    Frame,
};

///
pub struct PushComponent {
    visible: bool,
    force: bool,
    git_push: AsyncPush,
    progress: Option<RemoteProgress>,
    pending: bool,
    branch: String,
    queue: Queue,
    theme: SharedTheme,
    key_config: SharedKeyConfig,
    input_cred: CredComponent,
}

impl PushComponent {
    ///
    pub fn new(
        queue: &Queue,
        sender: &Sender<AsyncNotification>,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            queue: queue.clone(),
            force: false,
            pending: false,
            visible: false,
            branch: String::new(),
            git_push: AsyncPush::new(sender),
            progress: None,
            input_cred: CredComponent::new(
                theme.clone(),
                key_config.clone(),
            ),
            theme,
            key_config,
        }
    }

    ///
    pub fn push(
        &mut self,
        branch: String,
        force: bool,
    ) -> Result<()> {
        self.branch = branch;
        self.force = force;
        self.show()?;

        if need_username_password()? {
            let cred =
                extract_username_password().unwrap_or_else(|_| {
                    BasicAuthCredential::new(None, None)
                });
            if cred.is_complete() {
                self.push_to_remote(Some(cred), force)
            } else {
                self.input_cred.set_cred(cred);
                self.input_cred.show()
            }
        } else {
            self.push_to_remote(None, force)
        }
    }

    fn push_to_remote(
        &mut self,
        cred: Option<BasicAuthCredential>,
        force: bool,
    ) -> Result<()> {
        let remote = if let Some(remote) =
            get_branch_remote(CWD, &self.branch)?
        {
            log::info!("push: branch '{}' has upstream for remote '{}' - using that",self.branch,remote);
            remote
        } else {
            log::info!("push: branch '{}' has no upstream - looking up default remote",self.branch);
            let remote = get_default_remote(CWD)?;
            log::info!(
                "push: branch '{}' to remote '{}'",
                self.branch,
                remote
            );
            remote
        };

        self.pending = true;
        self.progress = None;
        self.git_push.request(PushRequest {
            remote,
            branch: self.branch.clone(),
            force,
            basic_credential: cred,
        })?;
        Ok(())
    }

    ///
    pub fn update_git(
        &mut self,
        ev: AsyncNotification,
    ) -> Result<()> {
        if self.is_visible() {
            if let AsyncNotification::Push = ev {
                self.update()?;
            }
        }

        Ok(())
    }

    ///
    fn update(&mut self) -> Result<()> {
        self.pending = self.git_push.is_pending()?;
        self.progress = self.git_push.progress()?;

        if !self.pending {
            if let Some(err) = self.git_push.last_result()? {
                self.queue.borrow_mut().push_back(
                    InternalEvent::ShowErrorMsg(format!(
                        "push failed:\n{}",
                        err
                    )),
                );
            }
            self.hide();
        }

        Ok(())
    }

    ///
    pub const fn any_work_pending(&self) -> bool {
        self.pending
    }

    ///
    pub fn get_progress(
        progress: &Option<RemoteProgress>,
    ) -> (String, u8) {
        progress.as_ref().map_or(
            (strings::PUSH_POPUP_PROGRESS_NONE.into(), 0),
            |progress| {
                (
                    Self::progress_state_name(&progress.state),
                    progress.get_progress_percent(),
                )
            },
        )
    }

    fn progress_state_name(state: &RemoteProgressState) -> String {
        match state {
            RemoteProgressState::PackingAddingObject => {
                strings::PUSH_POPUP_STATES_ADDING
            }
            RemoteProgressState::PackingDeltafiction => {
                strings::PUSH_POPUP_STATES_DELTAS
            }
            RemoteProgressState::Pushing => {
                strings::PUSH_POPUP_STATES_PUSHING
            }
            RemoteProgressState::Transfer => {
                strings::PUSH_POPUP_STATES_TRANSFER
            }
            RemoteProgressState::Done => {
                strings::PUSH_POPUP_STATES_DONE
            }
        }
        .into()
    }
}

impl DrawableComponent for PushComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        if self.visible {
            let (state, progress) =
                Self::get_progress(&self.progress);

            let area = ui::centered_rect_absolute(30, 3, f.size());

            f.render_widget(Clear, area);
            f.render_widget(
                Gauge::default()
                    .label(state.as_str())
                    .block(
                        Block::default()
                            .title(Span::styled(
                                if self.force {
                                    strings::FORCE_PUSH_POPUP_MSG
                                } else {
                                    strings::PUSH_POPUP_MSG
                                },
                                self.theme.title(true),
                            ))
                            .borders(Borders::ALL)
                            .border_type(BorderType::Thick)
                            .border_style(self.theme.block(true)),
                    )
                    .gauge_style(self.theme.push_gauge())
                    .percent(u16::from(progress)),
                area,
            );
            self.input_cred.draw(f, rect)?;
        }

        Ok(())
    }
}

impl Component for PushComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.is_visible() {
            out.clear();
        }

        if self.input_cred.is_visible() {
            self.input_cred.commands(out, force_all)
        } else {
            out.push(CommandInfo::new(
                strings::commands::close_msg(&self.key_config),
                !self.pending,
                self.visible,
            ));
            visibility_blocking(self)
        }
    }

    fn event(&mut self, ev: Event) -> Result<EventState> {
        if self.visible {
            if let Event::Key(e) = ev {
                if self.input_cred.is_visible() {
                    self.input_cred.event(ev)?;

                    if self.input_cred.get_cred().is_complete()
                        || !self.input_cred.is_visible()
                    {
                        self.push_to_remote(
                            Some(self.input_cred.get_cred().clone()),
                            self.force,
                        )?;
                        self.input_cred.hide();
                    }
                } else if e == self.key_config.exit_popup
                    && !self.pending
                {
                    self.hide();
                }
            }
            return Ok(EventState::Consumed);
        }
        Ok(EventState::NotConsumed)
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn show(&mut self) -> Result<()> {
        self.visible = true;

        Ok(())
    }
}
