use crate::{
    accessors,
    components::{
        command_pump, event_pump, visibility_blocking,
        ChangesComponent, CommandBlocking, CommandInfo, Component,
        DiffComponent, DrawableComponent, FileTreeItemKind,
    },
    keys,
    queue::{InternalEvent, Queue, ResetItem},
    strings::{self, commands, order},
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
    sync::{self, status::StatusType},
    AsyncDiff, AsyncNotification, AsyncStatus, DiffParams, DiffType,
    StatusParams, CWD,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use tui::layout::{Constraint, Direction, Layout};

///
#[derive(PartialEq)]
enum Focus {
    WorkDir,
    Diff,
    Stage,
}

///
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
    queue: Queue,
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
    ) -> Self {
        Self {
            queue: queue.clone(),
            visible: true,
            focus: Focus::WorkDir,
            diff_target: DiffTarget::WorkingDir,
            index_wd: ChangesComponent::new(
                strings::TITLE_STATUS,
                true,
                true,
                queue.clone(),
                theme.clone(),
            ),
            index: ChangesComponent::new(
                strings::TITLE_INDEX,
                false,
                false,
                queue.clone(),
                theme.clone(),
            ),
            diff: DiffComponent::new(Some(queue.clone()), theme),
            git_diff: AsyncDiff::new(sender.clone()),
            git_status_workdir: AsyncStatus::new(sender.clone()),
            git_status_stage: AsyncStatus::new(sender.clone()),
        }
    }

    fn can_focus_diff(&self) -> bool {
        match self.focus {
            Focus::WorkDir => self.index_wd.is_file_seleted(),
            Focus::Stage => self.index.is_file_seleted(),
            _ => false,
        }
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
        if self.is_visible() {
            self.git_diff.refresh()?;
            self.git_status_workdir.fetch(StatusParams::new(
                StatusType::WorkingDir,
                true,
            ))?;
            self.git_status_stage
                .fetch(StatusParams::new(StatusType::Stage, true))?;

            self.index_wd.update()?;
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
            _ => (),
        }

        Ok(())
    }

    fn update_status(&mut self) -> Result<()> {
        let status = self.git_status_stage.last()?;
        self.index.set_items(&status.items)?;

        let status = self.git_status_workdir.last()?;
        self.index_wd.set_items(&status.items)?;

        self.update_diff()?;

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
                        self.diff.update(path, is_stage, last)?;
                    }
                }
            } else {
                // we dont show the right diff right now, so we need to request
                if let Some(diff) =
                    self.git_diff.request(diff_params)?
                {
                    self.diff.update(path, is_stage, diff)?;
                } else {
                    self.diff.clear()?;
                }
            }
        } else {
            self.diff.clear()?;
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
}

impl Component for Status {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.visible || force_all {
            command_pump(
                out,
                force_all,
                self.components().as_slice(),
            );
        }

        {
            let focus_on_diff = self.focus == Focus::Diff;
            out.push(CommandInfo::new(
                commands::EDIT_ITEM,
                if focus_on_diff {
                    true
                } else {
                    self.can_focus_diff()
                },
                self.visible || force_all,
            ));
            out.push(CommandInfo::new(
                commands::DIFF_FOCUS_LEFT,
                true,
                (self.visible && focus_on_diff) || force_all,
            ));
            out.push(CommandInfo::new(
                commands::DIFF_FOCUS_RIGHT,
                self.can_focus_diff(),
                (self.visible && !focus_on_diff) || force_all,
            ));
        }

        out.push(
            CommandInfo::new(
                commands::SELECT_STATUS,
                true,
                (self.visible && self.focus == Focus::Diff)
                    || force_all,
            )
            .hidden(),
        );

        out.push(
            CommandInfo::new(
                commands::SELECT_STAGING,
                true,
                (self.visible && self.focus == Focus::WorkDir)
                    || force_all,
            )
            .order(order::NAV),
        );

        out.push(
            CommandInfo::new(
                commands::SELECT_UNSTAGED,
                true,
                (self.visible && self.focus == Focus::Stage)
                    || force_all,
            )
            .order(order::NAV),
        );

        visibility_blocking(self)
    }

    fn event(&mut self, ev: crossterm::event::Event) -> Result<bool> {
        if self.visible {
            if event_pump(ev, self.components_mut().as_mut_slice())? {
                return Ok(true);
            }

            if let Event::Key(k) = ev {
                return match k {
                    keys::FOCUS_WORKDIR => {
                        self.switch_focus(Focus::WorkDir)
                    }
                    keys::FOCUS_STAGE => {
                        self.switch_focus(Focus::Stage)
                    }
                    keys::EDIT_FILE
                        if self.can_focus_diff()
                            || self.focus == Focus::Diff =>
                    {
                        if let Some((path, _)) = self.selected_path()
                        {
                            self.queue.borrow_mut().push_back(
                                InternalEvent::OpenExternalEditor(
                                    Some(path),
                                ),
                            );
                        }
                        Ok(true)
                    }
                    keys::FOCUS_RIGHT if self.can_focus_diff() => {
                        self.switch_focus(Focus::Diff)
                    }
                    keys::FOCUS_LEFT => {
                        self.switch_focus(match self.diff_target {
                            DiffTarget::Stage => Focus::Stage,
                            DiffTarget::WorkingDir => Focus::WorkDir,
                        })
                    }
                    keys::MOVE_DOWN
                        if self.focus == Focus::WorkDir
                            && !self.index.is_empty() =>
                    {
                        self.switch_focus(Focus::Stage)
                    }

                    keys::MOVE_UP
                        if self.focus == Focus::Stage
                            && !self.index_wd.is_empty() =>
                    {
                        self.switch_focus(Focus::WorkDir)
                    }
                    _ => Ok(false),
                };
            }
        }

        Ok(false)
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
