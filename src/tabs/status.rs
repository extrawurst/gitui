use crate::{
    accessors,
    components::{
        command_pump, event_pump, visibility_blocking,
        ChangesComponent, CommandBlocking, CommandInfo, Component,
        DiffComponent, DrawableComponent, EventState,
        FileTreeItemKind,
    },
    keys::SharedKeyConfig,
    queue::{Action, InternalEvent, Queue, ResetItem},
    strings, try_or_popup,
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
    cached,
    sync::BranchCompare,
    sync::{self, status::StatusType, RepoState},
    AsyncDiff, AsyncNotification, AsyncStatus, DiffParams, DiffType,
    StatusParams, CWD,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use itertools::Itertools;
use std::convert::Into;
use std::convert::TryFrom;
use tui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::Paragraph,
};

/// what part of the screen is focused
#[derive(PartialEq)]
enum Focus {
    WorkDir,
    Diff,
    Stage,
}

/// focus can toggle between workdir and stage
impl Focus {
    const fn toggled_focus(&self) -> Self {
        match self {
            Self::WorkDir => Self::Stage,
            Self::Stage => Self::WorkDir,
            Self::Diff => Self::Diff,
        }
    }
}

/// which target are we showing a diff against
#[derive(PartialEq, Copy, Clone)]
enum DiffTarget {
    Stage,
    WorkingDir,
}

pub struct Status {
    visible: bool,
    focus: Focus,
    diff_target: DiffTarget,
    index: ChangesComponent,
    index_wd: ChangesComponent,
    diff: DiffComponent,
    git_diff: AsyncDiff,
    git_status_workdir: AsyncStatus,
    git_status_stage: AsyncStatus,
    git_branch_state: Option<BranchCompare>,
    git_branch_name: cached::BranchName,
    queue: Queue,
    git_action_executed: bool,
    key_config: SharedKeyConfig,
}

impl DrawableComponent for Status {
    fn draw<B: tui::backend::Backend>(
        &self,
        f: &mut tui::Frame<B>,
        rect: tui::layout::Rect,
    ) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                if self.focus == Focus::Diff {
                    [
                        Constraint::Percentage(30),
                        Constraint::Percentage(70),
                    ]
                } else {
                    [
                        Constraint::Percentage(50),
                        Constraint::Percentage(50),
                    ]
                }
                .as_ref(),
            )
            .split(rect);

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                if self.diff_target == DiffTarget::WorkingDir {
                    [
                        Constraint::Percentage(60),
                        Constraint::Percentage(40),
                    ]
                } else {
                    [
                        Constraint::Percentage(40),
                        Constraint::Percentage(60),
                    ]
                }
                .as_ref(),
            )
            .split(chunks[0]);

        self.index_wd.draw(f, left_chunks[0])?;
        self.index.draw(f, left_chunks[1])?;
        self.diff.draw(f, chunks[1])?;
        self.draw_branch_state(f, &left_chunks);
        Self::draw_repo_state(f, left_chunks[0])?;

        Ok(())
    }
}

impl Status {
    accessors!(self, [index, index_wd, diff]);

    ///
    pub fn new(
        queue: &Queue,
        sender: &Sender<AsyncNotification>,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            queue: queue.clone(),
            visible: true,
            focus: Focus::WorkDir,
            diff_target: DiffTarget::WorkingDir,
            index_wd: ChangesComponent::new(
                &strings::title_status(&key_config),
                true,
                true,
                queue.clone(),
                theme.clone(),
                key_config.clone(),
            ),
            index: ChangesComponent::new(
                &strings::title_index(&key_config),
                false,
                false,
                queue.clone(),
                theme.clone(),
                key_config.clone(),
            ),
            diff: DiffComponent::new(
                queue.clone(),
                theme,
                key_config.clone(),
                false,
            ),
            git_diff: AsyncDiff::new(sender),
            git_status_workdir: AsyncStatus::new(sender.clone()),
            git_status_stage: AsyncStatus::new(sender.clone()),
            git_action_executed: false,
            git_branch_state: None,
            git_branch_name: cached::BranchName::new(CWD),
            key_config,
        }
    }

    fn draw_branch_state<B: tui::backend::Backend>(
        &self,
        f: &mut tui::Frame<B>,
        chunks: &[tui::layout::Rect],
    ) {
        if let Some(branch_name) = self.git_branch_name.last() {
            let ahead_behind =
                if let Some(state) = &self.git_branch_state {
                    format!(
                        "\u{2191}{} \u{2193}{} ",
                        state.ahead, state.behind,
                    )
                } else {
                    String::new()
                };
            let w = Paragraph::new(format!(
                "{}{{{}}}",
                ahead_behind, branch_name
            ))
            .alignment(Alignment::Right);

            let mut rect = if self.index_wd.focused() {
                let mut rect = chunks[0];
                rect.y += rect.height.saturating_sub(1);
                rect
            } else {
                chunks[1]
            };

            rect.x += 1;
            rect.width = rect.width.saturating_sub(2);
            rect.height = rect
                .height
                .saturating_sub(rect.height.saturating_sub(1));

            f.render_widget(w, rect);
        }
    }

    fn draw_repo_state<B: tui::backend::Backend>(
        f: &mut tui::Frame<B>,
        r: tui::layout::Rect,
    ) -> Result<()> {
        if let Ok(state) = sync::repo_state(CWD) {
            if state != RepoState::Clean {
                let ids =
                    sync::mergehead_ids(CWD).unwrap_or_default();
                let ids = format!(
                    "({})",
                    ids.iter()
                        .map(|id| sync::CommitId::get_short_string(
                            id
                        ))
                        .join(",")
                );
                let txt = format!("{:?} {}", state, ids);
                let txt_len = u16::try_from(txt.len())?;
                let w = Paragraph::new(txt)
                    .style(Style::default().fg(Color::Red))
                    .alignment(Alignment::Left);

                let mut rect = r;
                rect.x += 1;
                rect.width =
                    rect.width.saturating_sub(2).min(txt_len);
                rect.y += rect.height.saturating_sub(1);
                rect.height = rect
                    .height
                    .saturating_sub(rect.height.saturating_sub(1));

                f.render_widget(w, rect);
            }
        }

        Ok(())
    }

    fn can_focus_diff(&self) -> bool {
        match self.focus {
            Focus::WorkDir => self.index_wd.is_file_seleted(),
            Focus::Stage => self.index.is_file_seleted(),
            Focus::Diff => false,
        }
    }

    fn is_focus_on_diff(&self) -> bool {
        self.focus == Focus::Diff
    }

    fn switch_focus(&mut self, f: Focus) -> Result<bool> {
        if self.focus != f {
            self.focus = f;

            match self.focus {
                Focus::WorkDir => {
                    self.set_diff_target(DiffTarget::WorkingDir);
                    self.diff.focus(false);
                }
                Focus::Stage => {
                    self.set_diff_target(DiffTarget::Stage);
                    self.diff.focus(false);
                }
                Focus::Diff => {
                    self.index.focus(false);
                    self.index_wd.focus(false);

                    self.diff.focus(true);
                }
            };

            self.update_diff()?;

            return Ok(true);
        }

        Ok(false)
    }

    fn set_diff_target(&mut self, target: DiffTarget) {
        self.diff_target = target;
        let is_stage = self.diff_target == DiffTarget::Stage;

        self.index_wd.focus_select(!is_stage);
        self.index.focus_select(is_stage);
    }

    pub fn selected_path(&self) -> Option<(String, bool)> {
        let (idx, is_stage) = match self.diff_target {
            DiffTarget::Stage => (&self.index, true),
            DiffTarget::WorkingDir => (&self.index_wd, false),
        };

        if let Some(item) = idx.selection() {
            if let FileTreeItemKind::File(i) = item.kind {
                return Some((i.path, is_stage));
            }
        }
        None
    }

    ///
    pub fn update(&mut self) -> Result<()> {
        self.git_branch_name.lookup().map(Some).unwrap_or(None);

        if self.is_visible() {
            self.git_diff.refresh()?;
            self.git_status_workdir
                .fetch(&StatusParams::new(StatusType::WorkingDir))?;
            self.git_status_stage
                .fetch(&StatusParams::new(StatusType::Stage))?;

            self.branch_compare();
        }

        Ok(())
    }

    ///
    pub fn anything_pending(&self) -> bool {
        self.git_diff.is_pending()
            || self.git_status_stage.is_pending()
            || self.git_status_workdir.is_pending()
    }

    ///
    pub fn update_git(
        &mut self,
        ev: AsyncNotification,
    ) -> Result<()> {
        match ev {
            AsyncNotification::Diff => self.update_diff()?,
            AsyncNotification::Status => self.update_status()?,
            AsyncNotification::Push
            | AsyncNotification::Fetch
            | AsyncNotification::CommitFiles => self.branch_compare(),
            _ => (),
        }

        Ok(())
    }

    fn update_status(&mut self) -> Result<()> {
        let stage_status = self.git_status_stage.last()?;
        self.index.set_items(&stage_status.items)?;

        let workdir_status = self.git_status_workdir.last()?;
        self.index_wd.set_items(&workdir_status.items)?;

        self.update_diff()?;

        if self.git_action_executed {
            self.git_action_executed = false;

            if self.focus == Focus::WorkDir
                && workdir_status.items.is_empty()
                && !stage_status.items.is_empty()
            {
                self.switch_focus(Focus::Stage)?;
            } else if self.focus == Focus::Stage
                && stage_status.items.is_empty()
            {
                self.switch_focus(Focus::WorkDir)?;
            }
        }

        Ok(())
    }

    ///
    pub fn update_diff(&mut self) -> Result<()> {
        if let Some((path, is_stage)) = self.selected_path() {
            let diff_type = if is_stage {
                DiffType::Stage
            } else {
                DiffType::WorkDir
            };

            let diff_params = DiffParams {
                path: path.clone(),
                diff_type,
            };

            if self.diff.current() == (path.clone(), is_stage) {
                // we are already showing a diff of the right file
                // maybe the diff changed (outside file change)
                if let Some((params, last)) = self.git_diff.last()? {
                    if params == diff_params {
                        self.diff.update(path, is_stage, last);
                    }
                }
            } else {
                // we dont show the right diff right now, so we need to request
                if let Some(diff) =
                    self.git_diff.request(diff_params)?
                {
                    self.diff.update(path, is_stage, diff);
                } else {
                    self.diff.clear(true);
                }
            }
        } else {
            self.diff.clear(false);
        }

        Ok(())
    }

    /// called after confirmation
    pub fn reset(&mut self, item: &ResetItem) -> bool {
        if let Err(e) = sync::reset_workdir(CWD, item.path.as_str()) {
            self.queue.borrow_mut().push_back(
                InternalEvent::ShowErrorMsg(format!(
                    "reset failed:\n{}",
                    e
                )),
            );

            false
        } else {
            true
        }
    }

    pub fn last_file_moved(&mut self) -> Result<()> {
        if !self.is_focus_on_diff() && self.is_visible() {
            self.switch_focus(self.focus.toggled_focus())?;
        }
        Ok(())
    }

    fn push(&self, force: bool) {
        if self.can_push() {
            if let Some(branch) = self.git_branch_name.last() {
                if force {
                    self.queue.borrow_mut().push_back(
                        InternalEvent::ConfirmAction(
                            Action::ForcePush(branch, force),
                        ),
                    );
                } else {
                    self.queue.borrow_mut().push_back(
                        InternalEvent::Push(branch, force),
                    );
                }
            }
        }
    }

    fn pull(&self) {
        if let Some(branch) = self.git_branch_name.last() {
            self.queue
                .borrow_mut()
                .push_back(InternalEvent::Pull(branch));
        }
    }

    fn branch_compare(&mut self) {
        self.git_branch_state =
            self.git_branch_name.last().and_then(|branch| {
                sync::branch_compare_upstream(CWD, branch.as_str())
                    .ok()
            });
    }

    fn can_push(&self) -> bool {
        self.git_branch_state
            .as_ref()
            .map_or(true, |state| state.ahead > 0)
    }

    fn can_abort_merge() -> bool {
        sync::repo_state(CWD).unwrap_or(RepoState::Clean)
            == RepoState::Merge
    }

    pub fn abort_merge(&self) {
        try_or_popup!(self, "abort merge", sync::abort_merge(CWD));
    }

    fn commands_nav(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) {
        let focus_on_diff = self.is_focus_on_diff();
        out.push(
            CommandInfo::new(
                strings::commands::diff_focus_left(&self.key_config),
                true,
                (self.visible && focus_on_diff) || force_all,
            )
            .order(strings::order::NAV),
        );
        out.push(
            CommandInfo::new(
                strings::commands::diff_focus_right(&self.key_config),
                self.can_focus_diff(),
                (self.visible && !focus_on_diff) || force_all,
            )
            .order(strings::order::NAV),
        );
        out.push(
            CommandInfo::new(
                strings::commands::select_staging(&self.key_config),
                !focus_on_diff,
                (self.visible
                    && !focus_on_diff
                    && self.focus == Focus::WorkDir)
                    || force_all,
            )
            .order(strings::order::NAV),
        );
        out.push(
            CommandInfo::new(
                strings::commands::select_unstaged(&self.key_config),
                !focus_on_diff,
                (self.visible
                    && !focus_on_diff
                    && self.focus == Focus::Stage)
                    || force_all,
            )
            .order(strings::order::NAV),
        );
    }
}

impl Component for Status {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        let focus_on_diff = self.is_focus_on_diff();

        if self.visible || force_all {
            command_pump(
                out,
                force_all,
                self.components().as_slice(),
            );

            out.push(CommandInfo::new(
                strings::commands::open_branch_select_popup(
                    &self.key_config,
                ),
                true,
                !focus_on_diff,
            ));

            out.push(CommandInfo::new(
                strings::commands::status_push(&self.key_config),
                self.can_push(),
                !focus_on_diff,
            ));
            out.push(CommandInfo::new(
                strings::commands::status_force_push(
                    &self.key_config,
                ),
                true,
                self.can_push() && !focus_on_diff,
            ));
            out.push(CommandInfo::new(
                strings::commands::status_pull(&self.key_config),
                true,
                !focus_on_diff,
            ));

            out.push(CommandInfo::new(
                strings::commands::abort_merge(&self.key_config),
                true,
                Self::can_abort_merge() || force_all,
            ));
        }

        {
            out.push(CommandInfo::new(
                strings::commands::edit_item(&self.key_config),
                if focus_on_diff {
                    true
                } else {
                    self.can_focus_diff()
                },
                self.visible || force_all,
            ));

            out.push(
                CommandInfo::new(
                    strings::commands::select_status(
                        &self.key_config,
                    ),
                    true,
                    (self.visible && !focus_on_diff) || force_all,
                )
                .hidden(),
            );

            self.commands_nav(out, force_all);
        }

        visibility_blocking(self)
    }

    fn event(
        &mut self,
        ev: crossterm::event::Event,
    ) -> Result<EventState> {
        if self.visible {
            if event_pump(ev, self.components_mut().as_mut_slice())?
                .is_consumed()
            {
                self.git_action_executed = true;
                return Ok(EventState::Consumed);
            }

            if let Event::Key(k) = ev {
                return if k == self.key_config.edit_file
                    && (self.can_focus_diff()
                        || self.is_focus_on_diff())
                {
                    if let Some((path, _)) = self.selected_path() {
                        self.queue.borrow_mut().push_back(
                            InternalEvent::OpenExternalEditor(Some(
                                path,
                            )),
                        );
                    }
                    Ok(EventState::Consumed)
                } else if k == self.key_config.toggle_workarea
                    && !self.is_focus_on_diff()
                {
                    self.switch_focus(self.focus.toggled_focus())
                        .map(Into::into)
                } else if k == self.key_config.focus_right
                    && self.can_focus_diff()
                {
                    self.switch_focus(Focus::Diff).map(Into::into)
                } else if k == self.key_config.focus_left {
                    self.switch_focus(match self.diff_target {
                        DiffTarget::Stage => Focus::Stage,
                        DiffTarget::WorkingDir => Focus::WorkDir,
                    })
                    .map(Into::into)
                } else if k == self.key_config.move_down
                    && self.focus == Focus::WorkDir
                    && !self.index.is_empty()
                {
                    self.switch_focus(Focus::Stage).map(Into::into)
                } else if k == self.key_config.move_up
                    && self.focus == Focus::Stage
                    && !self.index_wd.is_empty()
                {
                    self.switch_focus(Focus::WorkDir).map(Into::into)
                } else if k == self.key_config.select_branch
                    && !self.is_focus_on_diff()
                {
                    self.queue
                        .borrow_mut()
                        .push_back(InternalEvent::SelectBranch);
                    Ok(EventState::Consumed)
                } else if k == self.key_config.force_push
                    && !self.is_focus_on_diff()
                    && self.can_push()
                {
                    self.push(true);
                    Ok(EventState::Consumed)
                } else if k == self.key_config.push
                    && !self.is_focus_on_diff()
                {
                    self.push(false);
                    Ok(EventState::Consumed)
                } else if k == self.key_config.pull
                    && !self.is_focus_on_diff()
                {
                    self.pull();
                    Ok(EventState::Consumed)
                } else if k == self.key_config.abort_merge
                    && Self::can_abort_merge()
                {
                    self.queue.borrow_mut().push_back(
                        InternalEvent::ConfirmAction(
                            Action::AbortMerge,
                        ),
                    );

                    Ok(EventState::Consumed)
                } else {
                    Ok(EventState::NotConsumed)
                };
            }
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
        self.update()?;

        Ok(())
    }
}
