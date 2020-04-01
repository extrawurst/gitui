use crate::{
    components::{
        ChangesComponent, CommandInfo, CommitComponent, Component,
        DiffComponent, DrawableComponent, EventUpdate, HelpComponent,
    },
    keys, strings,
};
use asyncgit::{
    current_tick, sync, AsyncDiff, AsyncNotification, AsyncStatus,
    DiffParams, CWD,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use itertools::Itertools;
use log::trace;
use std::{borrow::Cow, path::Path};
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Tabs, Text, Widget},
    Frame,
};

///
#[derive(PartialEq)]
enum DiffTarget {
    Stage,
    WorkingDir,
}

///
#[derive(PartialEq)]
enum Focus {
    WorkDir,
    Diff,
    Stage,
}

///
pub struct App {
    focus: Focus,
    diff_target: DiffTarget,
    do_quit: bool,
    commit: CommitComponent,
    help: HelpComponent,
    index: ChangesComponent,
    index_wd: ChangesComponent,
    diff: DiffComponent,
    git_diff: AsyncDiff,
    git_status: AsyncStatus,
}

// public interface
impl App {
    ///
    pub fn new(sender: Sender<AsyncNotification>) -> Self {
        Self {
            focus: Focus::WorkDir,
            diff_target: DiffTarget::WorkingDir,
            do_quit: false,
            commit: CommitComponent::default(),
            help: HelpComponent::default(),
            index_wd: ChangesComponent::new(
                strings::TITLE_STATUS,
                true,
            ),
            index: ChangesComponent::new(strings::TITLE_INDEX, false),
            diff: DiffComponent::default(),
            git_diff: AsyncDiff::new(sender.clone()),
            git_status: AsyncStatus::new(sender),
        }
    }

    ///
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>) {
        let chunks_main = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(2),
                    Constraint::Min(2),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(f.size());

        Tabs::default()
            .block(Block::default().borders(Borders::BOTTOM))
            .titles(&[strings::TAB_STATUS])
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow))
            .divider(strings::TAB_DIVIDER)
            .render(f, chunks_main[0]);

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
            .split(chunks_main[1]);

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ]
                .as_ref(),
            )
            .split(chunks[0]);

        self.index_wd.draw(f, left_chunks[0]);
        self.index.draw(f, left_chunks[1]);
        self.diff.draw(f, chunks[1]);

        Self::draw_commands(
            f,
            chunks_main[2],
            self.commands().as_slice(),
        );

        self.commit.draw(f, f.size());
        self.help.draw(f, f.size());
    }

    ///
    pub fn event(&mut self, ev: Event) {
        trace!("event: {:?}", ev);

        if let Some(e) =
            Self::event_pump(ev, self.components().as_mut_slice())
        {
            match e {
                EventUpdate::Changes => self.update(),
                EventUpdate::Diff => self.update_diff(),
                _ => (),
            }

            return;
        }

        if let Event::Key(k) = ev {
            match k {
                keys::EXIT_1 | keys::EXIT_2 => self.do_quit = true,
                keys::FOCUS_WORKDIR => {
                    self.switch_focus(Focus::WorkDir)
                }
                keys::FOCUS_STAGE => self.switch_focus(Focus::Stage),
                keys::FOCUS_RIGHT => self.switch_focus(Focus::Diff),
                keys::FOCUS_LEFT => {
                    self.switch_focus(match self.diff_target {
                        DiffTarget::Stage => Focus::Stage,
                        DiffTarget::WorkingDir => Focus::WorkDir,
                    })
                }
                keys::OPEN_HELP => self.help.show(),
                keys::STATUS_STAGE_FILE => {
                    if self.index_add_remove() {
                        self.update();
                    }
                }
                keys::STATUS_RESET_FILE => {
                    self.index_reset();
                    self.update();
                }
                keys::OPEN_COMMIT if !self.index.is_empty() => {
                    self.commit.show();
                }
                _ => (),
            };
        }
    }

    ///
    pub fn update(&mut self) {
        trace!("update");

        self.git_diff.refresh();
        self.git_status.fetch(current_tick());
    }

    ///
    pub fn update_git(&mut self, ev: AsyncNotification) {
        trace!("update_git: {:?}", ev);
        match ev {
            AsyncNotification::Diff => self.update_diff(),
            AsyncNotification::Status => self.update_status(),
        }
    }

    ///
    pub fn is_quit(&self) -> bool {
        self.do_quit
    }
}

// private impls
impl App {
    fn update_diff(&mut self) {
        let (idx, is_stage) = match self.diff_target {
            DiffTarget::Stage => (&self.index, true),
            DiffTarget::WorkingDir => (&self.index_wd, false),
        };

        if let Some(i) = idx.selection() {
            let path = i.path;
            let diff_params = DiffParams(path.clone(), is_stage);

            if self.diff.current() == (path.clone(), is_stage) {
                // we are already showing a diff of the right file
                // maybe the diff changed (outside file change)
                if let Some(last) = self.git_diff.last() {
                    self.diff.update(path, is_stage, last);
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

    fn update_status(&mut self) {
        let status = self.git_status.last();
        self.index.update(&status.stage);
        self.index_wd.update(&status.work_dir);

        self.update_diff();

        self.help.set_cmds(self.commands());
    }

    fn commands(&self) -> Vec<CommandInfo> {
        let mut res = self.commit.commands();
        res.extend(self.help.commands());

        let main_cmds_available =
            !self.commit.is_visible() && !self.help.is_visible();

        if main_cmds_available {
            res.extend(self.index.commands());
            res.extend(self.index_wd.commands());
            res.extend(self.diff.commands());
        }

        {
            res.push(CommandInfo::new(
                strings::COMMIT_CMD_OPEN,
                !self.index.is_empty(),
                main_cmds_available,
            ));

            {
                let main_cmds_index_wd_focused_availale =
                    main_cmds_available && self.index_wd.focused();

                let some_selection =
                    self.index_wd.selection().is_some();
                res.push(CommandInfo::new(
                    strings::CMD_STATUS_STAGE,
                    some_selection,
                    main_cmds_index_wd_focused_availale,
                ));
                res.push(CommandInfo::new(
                    strings::CMD_STATUS_RESET,
                    some_selection,
                    main_cmds_index_wd_focused_availale,
                ));

                res.push(CommandInfo::new(
                    strings::CMD_STATUS_UNSTAGE,
                    self.index.selection().is_some(),
                    main_cmds_available && self.index.focused(),
                ));
            }

            {
                let focus_on_stage = self.focus == Focus::Stage;
                let focus_not_diff = self.focus != Focus::Diff;
                res.push(CommandInfo::new_hidden(
                    strings::CMD_STATUS_FOCUS_UNSTAGED,
                    true,
                    main_cmds_available
                        && focus_on_stage
                        && !focus_not_diff,
                ));
                res.push(CommandInfo::new_hidden(
                    strings::CMD_STATUS_FOCUS_STAGED,
                    true,
                    main_cmds_available
                        && !focus_on_stage
                        && !focus_not_diff,
                ));
            }
            {
                let focus_on_diff = self.focus == Focus::Diff;
                res.push(CommandInfo::new(
                    strings::CMD_STATUS_LEFT,
                    true,
                    main_cmds_available && focus_on_diff,
                ));
                res.push(CommandInfo::new(
                    strings::CMD_STATUS_RIGHT,
                    true,
                    main_cmds_available && !focus_on_diff,
                ));
            }
            res.push(CommandInfo::new(
                strings::CMD_STATUS_HELP,
                true,
                main_cmds_available,
            ));
            res.push(CommandInfo::new(
                strings::CMD_STATUS_QUIT,
                true,
                main_cmds_available,
            ));
        }

        res
    }

    fn components(&mut self) -> Vec<&mut dyn Component> {
        vec![
            &mut self.help,
            &mut self.commit,
            &mut self.index,
            &mut self.index_wd,
            &mut self.diff,
        ]
    }

    fn event_pump(
        ev: Event,
        components: &mut [&mut dyn Component],
    ) -> Option<EventUpdate> {
        for c in components {
            if let Some(u) = c.event(ev) {
                return Some(u);
            }
        }

        None
    }

    fn draw_commands<B: Backend>(
        f: &mut Frame<B>,
        r: Rect,
        cmds: &[CommandInfo],
    ) {
        let splitter = Text::Styled(
            Cow::from(strings::CMD_SPLITTER),
            Style::default().bg(Color::Black),
        );

        let style_enabled =
            Style::default().fg(Color::White).bg(Color::Blue);

        let style_disabled =
            Style::default().fg(Color::DarkGray).bg(Color::Blue);
        let texts = cmds
            .iter()
            .filter_map(|c| {
                if c.show_in_quickbar() {
                    Some(Text::Styled(
                        Cow::from(c.name.clone()),
                        if c.enabled {
                            style_enabled
                        } else {
                            style_disabled
                        },
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        Paragraph::new(texts.iter().intersperse(&splitter))
            .alignment(Alignment::Left)
            .render(f, r);
    }

    fn switch_focus(&mut self, f: Focus) {
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
        }
    }

    fn set_diff_target(&mut self, target: DiffTarget) {
        self.diff_target = target;
        let is_stage = self.diff_target == DiffTarget::Stage;

        self.index_wd.focus_select(!is_stage);
        self.index.focus_select(is_stage);
    }

    fn index_add_remove(&mut self) -> bool {
        if self.diff_target == DiffTarget::WorkingDir {
            if let Some(i) = self.index_wd.selection() {
                let path = Path::new(i.path.as_str());

                return sync::stage_add(CWD, path);
            }
        } else if let Some(i) = self.index.selection() {
            let path = Path::new(i.path.as_str());

            return sync::reset_stage(CWD, path);
        }

        false
    }

    fn index_reset(&mut self) {
        if self.index_wd.focused() {
            if let Some(i) = self.index_wd.selection() {
                let path = Path::new(i.path.as_str());

                if sync::reset_workdir(CWD, path) {
                    self.update();
                }
            }
        }
    }
}
