use crate::{
    components::{
        visibility_blocking, CommandBlocking, CommandInfo,
        CommitDetailsComponent, CommitList, Component,
        DrawableComponent,
    },
    keys::SharedKeyConfig,
    queue::{InternalEvent, Queue},
    strings,
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
    cached,
    sync::{self, CommitId},
    AsyncLog, AsyncNotification, AsyncTags, FetchStatus, CWD,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use std::time::Duration;
use sync::CommitTags;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

const SLICE_SIZE: usize = 1200;

///
pub struct Revlog {
    commit_details: CommitDetailsComponent,
    list: CommitList,
    git_log: AsyncLog,
    git_tags: AsyncTags,
    queue: Queue,
    visible: bool,
    branch_name: cached::BranchName,
    key_config: SharedKeyConfig,
}

impl Revlog {
    ///
    pub fn new(
        queue: &Queue,
        sender: &Sender<AsyncNotification>,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            queue: queue.clone(),
            commit_details: CommitDetailsComponent::new(
                queue,
                sender,
                theme.clone(),
                key_config.clone(),
            ),
            list: CommitList::new(
                &strings::log_title(&key_config),
                theme,
                key_config.clone(),
            ),
            git_log: AsyncLog::new(sender),
            git_tags: AsyncTags::new(sender),
            visible: false,
            branch_name: cached::BranchName::new(CWD),
            key_config,
        }
    }

    ///
    pub fn any_work_pending(&self) -> bool {
        self.git_log.is_pending()
            || self.git_tags.is_pending()
            || self.commit_details.any_work_pending()
    }

    ///
    pub fn update(&mut self) -> Result<()> {
        if self.visible {
            let log_changed =
                self.git_log.fetch()? == FetchStatus::Started;

            self.list.set_count_total(self.git_log.count()?);

            let selection = self.list.selection();
            let selection_max = self.list.selection_max();
            if self.list.items().needs_data(selection, selection_max)
                || log_changed
            {
                self.fetch_commits()?;
            }

            self.git_tags.request(Duration::from_secs(3), false)?;

            self.list.set_branch(
                self.branch_name.lookup().map(Some).unwrap_or(None),
            );

            if self.commit_details.is_visible() {
                let commit = self.selected_commit();
                let tags = self.selected_commit_tags(&commit);

                self.commit_details.set_commit(commit, tags)?;
            }
        }

        Ok(())
    }

    ///
    pub fn update_git(
        &mut self,
        ev: AsyncNotification,
    ) -> Result<()> {
        if self.visible {
            match ev {
                AsyncNotification::CommitFiles
                | AsyncNotification::Log => self.update()?,
                AsyncNotification::Tags => {
                    if let Some(tags) = self.git_tags.last()? {
                        self.list.set_tags(tags);
                        self.update()?;
                    }
                }
                _ => (),
            }
        }

        Ok(())
    }

    fn fetch_commits(&mut self) -> Result<()> {
        let want_min =
            self.list.selection().saturating_sub(SLICE_SIZE / 2);

        let commits = sync::get_commits_info(
            CWD,
            &self.git_log.get_slice(want_min, SLICE_SIZE)?,
            self.list.current_size().0.into(),
        );

        if let Ok(commits) = commits {
            self.list.items().set_items(want_min, commits);
        }

        Ok(())
    }

    fn selected_commit(&self) -> Option<CommitId> {
        self.list.selected_entry().map(|e| e.id)
    }

    fn copy_commit_hash(&self) -> Result<()> {
        self.list.copy_entry_hash()?;
        Ok(())
    }

    fn selected_commit_tags(
        &self,
        commit: &Option<CommitId>,
    ) -> Option<CommitTags> {
        let tags = self.list.tags();

        commit.and_then(|commit| {
            tags.and_then(|tags| tags.get(&commit).cloned())
        })
    }
}

impl DrawableComponent for Revlog {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        area: Rect,
    ) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage(60),
                    Constraint::Percentage(40),
                ]
                .as_ref(),
            )
            .split(area);

        if self.commit_details.is_visible() {
            self.list.draw(f, chunks[0])?;
            self.commit_details.draw(f, chunks[1])?;
        } else {
            self.list.draw(f, area)?;
        }

        Ok(())
    }
}

impl Component for Revlog {
    fn event(&mut self, ev: Event) -> Result<bool> {
        if self.visible {
            let event_used = self.list.event(ev)?;

            if event_used {
                self.update()?;
                return Ok(true);
            } else if let Event::Key(k) = ev {
                if k == self.key_config.enter {
                    self.commit_details.toggle_visible()?;
                    self.update()?;
                    return Ok(true);
                } else if k == self.key_config.copy {
                    self.copy_commit_hash()?;
                    return Ok(true);
                } else if k == self.key_config.log_tag_commit
                    || (k == self.key_config.focus_right
                        && self.commit_details.is_visible())
                {
                    return self.selected_commit().map_or(
                        Ok(false),
                        |id| {
                            self.queue.borrow_mut().push_back(
                                InternalEvent::InspectCommit(
                                    id,
                                    self.selected_commit_tags(&Some(
                                        id,
                                    )),
                                ),
                            );
                            Ok(true)
                        },
                    );
                } else if k == self.key_config.enter
                    || k == self.key_config.focus_right
                {
                    self.commit_details.toggle_visible()?;
                    self.update()?;
                    return Ok(true);
                } else if k == self.key_config.log_tag_commit {
                    return self.selected_commit().map_or(
                        Ok(false),
                        |id| {
                            self.queue.borrow_mut().push_back(
                                InternalEvent::TagCommit(id),
                            );
                            Ok(true)
                        },
                    );
                } else if k == self.key_config.focus_left
                    && self.commit_details.is_visible()
                {
                    self.commit_details.hide();
                } else if k == self.key_config.select_branch {
                    self.queue
                        .borrow_mut()
                        .push_back(InternalEvent::SelectBranch);
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.visible || force_all {
            self.list.commands(out, force_all);
        }

        out.push(CommandInfo::new(
            strings::commands::log_details_toggle(&self.key_config),
            true,
            self.visible,
        ));

        out.push(CommandInfo::new(
            strings::commands::log_details_open(&self.key_config),
            true,
            (self.visible && self.commit_details.is_visible())
                || force_all,
        ));

        out.push(CommandInfo::new(
            strings::commands::log_tag_commit(&self.key_config),
            true,
            self.visible || force_all,
        ));

        out.push(CommandInfo::new(
            strings::commands::open_branch_select_popup(
                &self.key_config,
            ),
            true,
            self.visible || force_all,
        ));

        out.push(CommandInfo::new(
            strings::commands::copy_hash(&self.key_config),
            true,
            self.visible || force_all,
        ));

        visibility_blocking(self)
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hide(&mut self) {
        self.visible = false;
        self.git_log.set_background();
    }

    fn show(&mut self) -> Result<()> {
        self.visible = true;
        self.list.clear();
        self.update()?;

        Ok(())
    }
}
