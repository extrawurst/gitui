use crate::components::TextInputComponent;
use crate::{
    components::{
        visibility_blocking, CommandBlocking, CommandInfo, Component,
        DrawableComponent,
    },
    keys::SharedKeyConfig,
    queue::{InternalEvent, Queue},
    strings,
    ui::{self, style::SharedTheme},
};
use anyhow::Result;
use asyncgit::{
    sync::cred::extract_username_password,
    sync::cred::need_username_password,
    sync::cred::BasicAuthCredential, AsyncNotification, AsyncPush,
    PushProgress, PushProgressState, PushRequest,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Clear, Gauge},
    Frame,
};

///
pub struct PushComponent {
    visible: bool,
    git_push: AsyncPush,
    progress: Option<PushProgress>,
    pending: bool,
    branch: String,
    queue: Queue,
    theme: SharedTheme,
    key_config: SharedKeyConfig,
    input_username: TextInputComponent,
    input_password: TextInputComponent,
    cred: Option<BasicAuthCredential>,
}

const DEFAULT_REMOTE_NAME: &str = "origin";

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
            pending: false,
            visible: false,
            branch: "".to_string(),
            git_push: AsyncPush::new(sender),
            progress: None,
            input_username: TextInputComponent::new(
                theme.clone(),
                key_config.clone(),
                &strings::username_popup_title(&key_config),
                &strings::username_popup_msg(&key_config),
            ),
            input_password: TextInputComponent::new(
                theme.clone(),
                key_config.clone(),
                &strings::password_popup_title(&key_config),
                &strings::password_popup_msg(&key_config),
            ),
            theme,
            key_config,
            cred: None,
        }
    }

    ///
    pub fn push(&mut self, branch: String) -> Result<()> {
        self.branch = branch;
        self.show()?;
        match need_username_password(DEFAULT_REMOTE_NAME) {
            Ok(true) => {
                self.cred = extract_username_password("origin")
                    .map(Some)
                    .unwrap_or(None);
                match &self.cred {
                    None
                    | Some(BasicAuthCredential {
                        username: None,
                        password: _,
                    }) => self.input_username.show(),
                    Some(BasicAuthCredential {
                        username: _,
                        password: None,
                    }) => self.input_password.show(),
                    Some(_) => self.push_to_remote(),
                }
            }
            _ => self.push_to_remote(),
        }
    }

    fn push_to_remote(&mut self) -> Result<()> {
        self.pending = true;
        self.progress = None;
        self.git_push.request(PushRequest {
            //TODO: find tracking branch name
            remote: String::from(DEFAULT_REMOTE_NAME),
            branch: self.branch.clone(),
            basic_credential: self.cred.clone(),
        })?;
        self.cred = None;
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

    fn get_progress(&self) -> (String, u8) {
        self.progress.as_ref().map_or(
            (strings::PUSH_POPUP_PROGRESS_NONE.into(), 0),
            |progress| {
                (
                    Self::progress_state_name(&progress.state),
                    progress.progress,
                )
            },
        )
    }

    fn progress_state_name(state: &PushProgressState) -> String {
        match state {
            PushProgressState::PackingAddingObject => {
                strings::PUSH_POPUP_STATES_ADDING
            }
            PushProgressState::PackingDeltafiction => {
                strings::PUSH_POPUP_STATES_DELTAS
            }
            PushProgressState::Pushing => {
                strings::PUSH_POPUP_STATES_PUSHING
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
            let (state, progress) = self.get_progress();

            let area = ui::centered_rect_absolute(30, 3, f.size());

            f.render_widget(Clear, area);
            f.render_widget(
                Gauge::default()
                    .label(state.as_str())
                    .block(
                        Block::default()
                            .title(Span::styled(
                                strings::PUSH_POPUP_MSG,
                                self.theme.title(true),
                            ))
                            .borders(Borders::ALL)
                            .border_type(BorderType::Thick)
                            .border_style(self.theme.block(true)),
                    )
                    .gauge_style(
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::Black), // .modifier(Modifier::ITALIC),
                    )
                    .percent(u16::from(progress)),
                area,
            );
            self.input_username.draw(f, rect)?;
            self.input_password.draw(f, rect)?;
        }

        Ok(())
    }
}

impl Component for PushComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        _force_all: bool,
    ) -> CommandBlocking {
        if self.is_visible() {
            out.clear();
        }

        out.push(CommandInfo::new(
            strings::commands::close_msg(&self.key_config),
            !self.pending,
            self.visible,
        ));

        visibility_blocking(self)
    }

    fn event(&mut self, ev: Event) -> Result<bool> {
        if self.visible {
            if let Event::Key(e) = ev {
                if e == self.key_config.exit_popup {
                    self.hide();
                }
                if self.input_username.event(ev)?
                    || self.input_password.event(ev)?
                {
                    return Ok(true);
                } else if e == self.key_config.enter {
                    if self.input_username.is_visible() {
                        self.cred = Some(BasicAuthCredential {
                            username: Some(
                                self.input_username
                                    .get_text()
                                    .to_owned(),
                            ),
                            password: None,
                        });
                        self.input_username.hide();
                        self.input_password.show()?;
                    } else if self.input_password.is_visible() {
                        self.cred = Some(BasicAuthCredential {
                            username: self.cred.as_ref().and_then(
                                |cred| cred.username.clone(),
                            ),
                            password: Some(
                                self.input_password
                                    .get_text()
                                    .to_owned(),
                            ),
                        });
                        self.input_password.hide();
                        self.input_password.clear();

                        self.push_to_remote()?;
                        self.cred = None;
                    } else {
                        self.hide();
                    }
                }
            }
            return Ok(true);
        }
        Ok(false)
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hide(&mut self) {
        self.visible = false
    }

    fn show(&mut self) -> Result<()> {
        self.visible = true;

        Ok(())
    }
}
