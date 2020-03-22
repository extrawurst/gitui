use crate::{
    components::{
        CommandInfo, CommitComponent, Component, DiffComponent,
        IndexComponent,
    },
    git_utils, keys, strings,
};
use asyncgit::AsyncDiff;
use crossbeam_channel::Sender;
use crossterm::event::Event;
use git2::StatusShow;
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
    Status,
    Diff,
    Stage,
}

pub struct App {
    focus: Focus,
    diff_target: DiffTarget,
    do_quit: bool,
    commit: CommitComponent,
    index: IndexComponent,
    index_wd: IndexComponent,
    diff: DiffComponent,
    async_diff: AsyncDiff,
}

// public interface
impl App {
    ///
    pub fn new(sender: Sender<()>) -> Self {
        Self {
            focus: Focus::Status,
            diff_target: DiffTarget::WorkingDir,
            do_quit: false,
            commit: CommitComponent::default(),
            index_wd: IndexComponent::new(
                strings::TITLE_STATUS,
                StatusShow::Workdir,
                true,
            ),
            index: IndexComponent::new(
                strings::TITLE_INDEX,
                StatusShow::Index,
                false,
            ),
            diff: DiffComponent::default(),
            async_diff: AsyncDiff::new(sender),
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

        let mut cmds = self.commit.commands();
        if !self.commit.is_visible() {
            cmds.extend(self.index.commands());
            cmds.extend(self.index_wd.commands());
            cmds.extend(self.diff.commands());
        }
        cmds.extend(self.commands());

        self.draw_commands(f, chunks_main[2], cmds);

        self.commit.draw(f, f.size());
    }

    ///
    pub fn event(&mut self, ev: Event) {
        trace!("event: {:?}", ev);

        if self.commit.is_visible() && self.commit.event(ev) {
            if !self.commit.is_visible() {
                self.update();
            }
            return;
        }

        if !self.commit.is_visible() {
            if self.index.event(ev) {
                self.update_diff();
                return;
            }
            if self.index_wd.event(ev) {
                self.update_diff();
                return;
            }
            if self.diff.event(ev) {
                return;
            }

            if let Event::Key(k) = ev {
                match k {
                    keys::EXIT_1 | keys::EXIT_2 => {
                        self.do_quit = true
                    }
                    keys::FOCUS_STATUS => {
                        self.switch_focus(Focus::Status)
                    }
                    keys::FOCUS_STAGE => {
                        self.switch_focus(Focus::Stage)
                    }
                    keys::FOCUS_RIGHT => {
                        self.switch_focus(Focus::Diff)
                    }
                    keys::FOCUS_LEFT => {
                        self.switch_focus(match self.diff_target {
                            DiffTarget::Stage => Focus::Stage,
                            DiffTarget::WorkingDir => Focus::Status,
                        })
                    }
                    keys::STATUS_STAGE_FILE => {
                        self.index_add_remove();
                        self.update();
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
    }

    ///
    pub fn update(&mut self) {
        trace!("app::update");

        self.index.update();
        self.index_wd.update();
        self.update_diff();
    }

    ///
    pub fn is_quit(&self) -> bool {
        self.do_quit
    }
}

impl App {
    pub fn update_diff(&mut self) {
        let (idx, is_stage) = match self.diff_target {
            DiffTarget::Stage => (&self.index, true),
            DiffTarget::WorkingDir => (&self.index_wd, false),
        };

        if let Some(i) = idx.selection() {
            let path = i.path;

            if self.diff.path() != path {
                if let Some(diff) =
                    self.async_diff.request(path.clone(), is_stage)
                {
                    self.diff.update(path.clone(), diff);
                } else {
                    self.diff.clear();
                }
            }
        } else {
            self.diff.clear();
        }
    }

    fn commands(&self) -> Vec<CommandInfo> {
        let mut res = Vec::new();
        if !self.commit.is_visible() {
            res.push(CommandInfo {
                name: strings::COMMIT_CMD_OPEN.to_string(),
                enabled: !self.index.is_empty(),
            });

            if self.index_wd.focused() {
                let some_selection =
                    self.index_wd.selection().is_some();
                res.push(CommandInfo {
                    name: strings::CMD_STATUS_STAGE.to_string(),
                    enabled: some_selection,
                });
                res.push(CommandInfo {
                    name: strings::CMD_STATUS_RESET.to_string(),
                    enabled: some_selection,
                });
            } else if self.index.focused() {
                res.push(CommandInfo {
                    name: strings::CMD_STATUS_UNSTAGE.to_string(),
                    enabled: self.index.selection().is_some(),
                });
            }

            res.push(CommandInfo {
                name: if self.focus == Focus::Diff {
                    strings::CMD_STATUS_LEFT.to_string()
                } else {
                    strings::CMD_STATUS_RIGHT.to_string()
                },
                enabled: true,
            });
            res.push(CommandInfo {
                name: strings::CMD_STATUS_QUIT.to_string(),
                enabled: true,
            });
        }

        res
    }

    fn draw_commands<B: Backend>(
        &self,
        f: &mut Frame<B>,
        r: Rect,
        cmds: Vec<CommandInfo>,
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
            .map(|c| {
                Text::Styled(
                    Cow::from(c.name.clone()),
                    if c.enabled {
                        style_enabled
                    } else {
                        style_disabled
                    },
                )
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
                Focus::Status => {
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

    fn index_add_remove(&mut self) {
        if self.diff_target == DiffTarget::WorkingDir {
            if let Some(i) = self.index_wd.selection() {
                let path = Path::new(i.path.as_str());

                if git_utils::stage_add(path) {
                    self.update();
                }
            }
        } else {
            if let Some(i) = self.index.selection() {
                let path = Path::new(i.path.as_str());

                if git_utils::stage_reset(path) {
                    self.update();
                }
            }
        }
    }

    fn index_reset(&mut self) {
        if self.index_wd.focused() {
            if let Some(i) = self.index_wd.selection() {
                let path = Path::new(i.path.as_str());

                if git_utils::index_reset(path) {
                    self.update();
                }
            }
        }
    }
}
