use super::PushComponent;
use crate::{
    components::{
        cred::CredComponent, visibility_blocking, CommandBlocking,
        CommandInfo, Component, DrawableComponent, EventState,
    },
    keys::SharedKeyConfig,
    queue::{Action, InternalEvent, Queue},
    strings, try_or_popup,
    ui::{self, style::SharedTheme},
};
use anyhow::Result;
use asyncgit::{
    sync::{
        self,
        cred::{
            extract_username_password, need_username_password,
            BasicAuthCredential,
        },
        get_default_remote,
    },
    AsyncFetch, AsyncNotification, FetchRequest, RemoteProgress, CWD,
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
pub struct PullComponent {
    visible: bool,
    git_fetch: AsyncFetch,
    progress: Option<RemoteProgress>,
    pending: bool,
    branch: String,
    queue: Queue,
    theme: SharedTheme,
    key_config: SharedKeyConfig,
    input_cred: CredComponent,
}

impl PullComponent {
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
            branch: String::new(),
            git_fetch: AsyncFetch::new(sender),
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
    pub fn fetch(&mut self, branch: String) -> Result<()> {
        self.branch = branch;
        self.show()?;
        if need_username_password()? {
            let cred =
                extract_username_password().unwrap_or_else(|_| {
                    BasicAuthCredential::new(None, None)
                });
            if cred.is_complete() {
                self.fetch_from_remote(Some(cred))
            } else {
                self.input_cred.set_cred(cred);
                self.input_cred.show()
            }
        } else {
            self.fetch_from_remote(None)
        }
    }

    fn fetch_from_remote(
        &mut self,
        cred: Option<BasicAuthCredential>,
    ) -> Result<()> {
        self.pending = true;
        self.progress = None;
        self.git_fetch.request(FetchRequest {
            remote: get_default_remote(CWD)?,
            branch: self.branch.clone(),
            basic_credential: cred,
        })?;

        Ok(())
    }

    ///
    pub const fn any_work_pending(&self) -> bool {
        self.pending
    }

    ///
    pub fn update_git(
        &mut self,
        ev: AsyncNotification,
    ) -> Result<()> {
        if self.is_visible() {
            if let AsyncNotification::Fetch = ev {
                self.update()?;
            }
        }

        Ok(())
    }

    ///
    fn update(&mut self) -> Result<()> {
        self.pending = self.git_fetch.is_pending()?;
        self.progress = self.git_fetch.progress()?;

        if !self.pending {
            if let Some((_bytes, err)) =
                self.git_fetch.last_result()?
            {
                if err.is_empty() {
                    self.try_ff_merge()?;
                } else {
                    self.pending = false;
                    self.hide();
                    self.queue.borrow_mut().push_back(
                        InternalEvent::ShowErrorMsg(format!(
                            "fetch failed:\n{}",
                            err
                        )),
                    );
                }
            }
        }

        Ok(())
    }

    // check if something is incoming and try a ff merge then
    fn try_ff_merge(&mut self) -> Result<()> {
        let branch_compare =
            sync::branch_compare_upstream(CWD, &self.branch)?;
        if branch_compare.behind > 0 {
            let ff_res = sync::branch_merge_upstream_fastforward(
                CWD,
                &self.branch,
            );
            if let Err(err) = ff_res {
                log::trace!("ff failed: {}", err);
                self.confirm_merge(branch_compare.behind);
            }
        }

        self.hide();

        Ok(())
    }

    pub fn try_conflict_free_merge(&self, rebase: bool) {
        if rebase {
            try_or_popup!(
                self,
                "rebase failed:",
                sync::merge_upstream_rebase(CWD, &self.branch)
            );
        } else {
            try_or_popup!(
                self,
                "merge failed:",
                sync::merge_upstream_commit(CWD, &self.branch)
            );
        }
    }

    fn confirm_merge(&mut self, incoming: usize) {
        self.queue.borrow_mut().push_back(
            InternalEvent::ConfirmAction(Action::PullMerge {
                incoming,
                rebase: sync::config_is_pull_rebase(CWD)
                    .unwrap_or_default(),
            }),
        );
        self.hide();
    }
}

impl DrawableComponent for PullComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        if self.visible {
            let (state, progress) =
                PushComponent::get_progress(&self.progress);

            let area = ui::centered_rect_absolute(30, 3, f.size());

            f.render_widget(Clear, area);
            f.render_widget(
                Gauge::default()
                    .label(state.as_str())
                    .block(
                        Block::default()
                            .title(Span::styled(
                                strings::PULL_POPUP_MSG,
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

impl Component for PullComponent {
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
            if let Event::Key(_) = ev {
                if self.input_cred.is_visible() {
                    self.input_cred.event(ev)?;

                    if self.input_cred.get_cred().is_complete()
                        || !self.input_cred.is_visible()
                    {
                        self.fetch_from_remote(Some(
                            self.input_cred.get_cred().clone(),
                        ))?;
                        self.input_cred.hide();
                    }
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
