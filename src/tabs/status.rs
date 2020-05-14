use crate::{
    accessors,
    components::{
        event_pump, ChangesComponent, CommandBlocking, CommandInfo,
        Component, DiffComponent, DrawableComponent,
        FileTreeItemKind,
    },
    keys,
    queue::Queue,
    strings,
};
use asyncgit::{
    current_tick, AsyncDiff, AsyncNotification, AsyncStatus,
    DiffParams,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use strings::commands;
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
    git_status: AsyncStatus,
}

impl DrawableComponent for Status {
    fn draw<B: tui::backend::Backend>(
        &mut self,
        f: &mut tui::Frame<B>,
        rect: tui::layout::Rect,
    ) {
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

        self.index_wd.draw(f, left_chunks[0]);
        self.index.draw(f, left_chunks[1]);
        self.diff.draw(f, chunks[1]);
    }
}

impl Status {
    accessors!(self, [index, index_wd, diff]);

    ///
    pub fn new(
        sender: &Sender<AsyncNotification>,
        queue: &Queue,
    ) -> Self {
        Self {
            visible: true,
            focus: Focus::WorkDir,
            diff_target: DiffTarget::WorkingDir,
            index_wd: ChangesComponent::new(
                strings::TITLE_STATUS,
                true,
                true,
                queue.clone(),
            ),
            index: ChangesComponent::new(
                strings::TITLE_INDEX,
                false,
                false,
                queue.clone(),
            ),
            diff: DiffComponent::new(queue.clone()),
            git_diff: AsyncDiff::new(sender.clone()),
            git_status: AsyncStatus::new(sender.clone()),
        }
    }

    fn can_focus_diff(&self) -> bool {
        match self.focus {
            Focus::WorkDir => self.index_wd.is_file_seleted(),
            Focus::Stage => self.index.is_file_seleted(),
            _ => false,
        }
    }

    fn switch_focus(&mut self, f: Focus) -> bool {
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

            self.update_diff();

            return true;
        }

        false
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
    pub fn update(&mut self) {
        self.git_diff.refresh();
        self.git_status.fetch(current_tick().unwrap()).unwrap();
    }

    ///
    pub fn anything_pending(&self) -> bool {
        self.git_diff.is_pending() || self.git_status.is_pending()
    }

    ///
    pub fn update_git(&mut self, ev: AsyncNotification) {
        match ev {
            AsyncNotification::Diff => self.update_diff(),
            AsyncNotification::Status => self.update_status(),
            _ => (),
        }
    }

    fn update_status(&mut self) {
        let status = self.git_status.last().unwrap();
        self.index.update(&status.stage);
        self.index_wd.update(&status.work_dir);

        self.update_diff();
    }

    ///
    pub fn update_diff(&mut self) {
        if let Some((path, is_stage)) = self.selected_path() {
            let diff_params = DiffParams(path.clone(), is_stage);

            if self.diff.current() == (path.clone(), is_stage) {
                // we are already showing a diff of the right file
                // maybe the diff changed (outside file change)
                if let Some((params, last)) =
                    self.git_diff.last().unwrap()
                {
                    if params == diff_params {
                        self.diff.update(path, is_stage, last);
                    }
                }
            } else {
                // we dont show the right diff right now, so we need to request
                if let Some(diff) = self.git_diff.request(diff_params)
                {
                    self.diff.update(path, is_stage, diff);
                } else {
                    self.diff.clear();
                }
            }
        } else {
            self.diff.clear();
        }
    }
}

impl Component for Status {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        for c in self.components() {
            if c.commands(out, force_all)
                != CommandBlocking::PassingOn
                && !force_all
            {
                break;
            }
        }

        {
            let focus_on_diff = self.focus == Focus::Diff;
            out.push(CommandInfo::new(
                commands::STATUS_FOCUS_LEFT,
                true,
                (self.visible && focus_on_diff) || force_all,
            ));
            out.push(CommandInfo::new(
                commands::STATUS_FOCUS_RIGHT,
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
            .order(-2),
        );

        out.push(
            CommandInfo::new(
                commands::SELECT_UNSTAGED,
                true,
                (self.visible && self.focus == Focus::Stage)
                    || force_all,
            )
            .order(-2),
        );

        if self.visible {
            CommandBlocking::Blocking
        } else {
            CommandBlocking::PassingOn
        }
    }

    fn event(&mut self, ev: crossterm::event::Event) -> bool {
        if self.visible {
            let conusmed =
                event_pump(ev, self.components_mut().as_mut_slice());

            if conusmed {
                return true;
            }

            if let Event::Key(k) = ev {
                return match k {
                    keys::FOCUS_WORKDIR => {
                        self.switch_focus(Focus::WorkDir)
                    }
                    keys::FOCUS_STAGE => {
                        self.switch_focus(Focus::Stage)
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
                    _ => false,
                };
            }
        }

        false
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn show(&mut self) {
        self.visible = true;
    }
}
