use crate::{
    components::{
        CommandInfo, CommitComponent, Component, DiffComponent,
        IndexComponent,
    },
    git_utils::{self, Diff},
};
use crossterm::event::{Event, KeyCode};
use git2::StatusShow;
use itertools::Itertools;
use std::{borrow::Cow, path::Path};
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Tabs, Text, Widget},
    Frame,
};

///
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
}

impl App {
    ///
    pub fn new() -> Self {
        Self {
            focus: Focus::Status,
            diff_target: DiffTarget::WorkingDir,
            do_quit: false,
            commit: CommitComponent::default(),
            index_wd: IndexComponent::new(
                "Status [s]",
                StatusShow::Workdir,
                true,
            ),
            index: IndexComponent::new(
                "Index [i]",
                StatusShow::Index,
                false,
            ),
            diff: DiffComponent::default(),
        }
    }
    ///
    pub fn is_quit(&self) -> bool {
        self.do_quit
    }
}

impl App {
    ///
    fn update_diff(&mut self) {
        let (idx, is_stage) = match self.diff_target {
            DiffTarget::Stage => (&self.index, true),
            DiffTarget::WorkingDir => (&self.index_wd, false),
        };

        let new_diff = match idx.selection() {
            Some(i) => git_utils::get_diff(
                Path::new(i.path.as_str()),
                is_stage,
            ),
            None => Diff::default(),
        };

        self.diff.update(new_diff);
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
            .titles(&["Status", "Branches", "Stash", "Misc"])
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow))
            .divider("  |  ")
            .render(f, chunks_main[0]);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ]
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

    fn commands(&self) -> Vec<CommandInfo> {
        let mut res = Vec::new();
        if !self.commit.is_visible() {
            if self.index_wd.focused() {
                let some_selection =
                    self.index_wd.selection().is_some();
                res.push(CommandInfo {
                    name: "Stage File [enter]".to_string(),
                    enabled: some_selection,
                });
                res.push(CommandInfo {
                    name: "Reset File [D]".to_string(),
                    enabled: some_selection,
                });
            } else if self.index.focused() {
                res.push(CommandInfo {
                    name: "Unstage File [enter]".to_string(),
                    enabled: self.index.selection().is_some(),
                });
            }

            res.push(CommandInfo {
                name: "Next [tab]".to_string(),
                enabled: true,
            });
            res.push(CommandInfo {
                name: "Quit [esc,q]".to_string(),
                enabled: true,
            });
        }

        res
    }

    ///
    pub fn event(&mut self, ev: Event) {
        if self.commit.event(ev) {
            return;
        }

        if !self.commit.is_visible() {
            if self.index.event(ev) {
                return;
            }
            if self.index_wd.event(ev) {
                return;
            }
            if self.diff.event(ev) {
                return;
            }

            if ev == Event::Key(KeyCode::Esc.into())
                || ev == Event::Key(KeyCode::Char('q').into())
            {
                self.do_quit = true;
            }

            if ev == Event::Key(KeyCode::Tab.into()) {
                self.toggle_focus();
            }

            if ev == Event::Key(KeyCode::Char('s').into()) {
                self.switch_focus(Focus::Status);
            } else if ev == Event::Key(KeyCode::Char('i').into()) {
                self.switch_focus(Focus::Stage);
            } else if ev == Event::Key(KeyCode::Char('d').into()) {
                self.switch_focus(Focus::Diff);
            }

            if let Event::Key(e) = ev {
                if e.code == KeyCode::Enter {
                    self.index_add_remove();
                }
            }

            if ev == Event::Key(KeyCode::Char('D').into()) {
                self.index_reset();
            }
        }
    }

    fn draw_commands<B: Backend>(
        &self,
        f: &mut Frame<B>,
        r: Rect,
        cmds: Vec<CommandInfo>,
    ) {
        let splitter = Text::Styled(
            Cow::from(" "),
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

    ///
    pub fn update(&mut self) {
        self.index.update();
        self.index_wd.update();
        self.update_diff();
    }

    fn toggle_focus(&mut self) {
        self.switch_focus(match self.focus {
            Focus::Status => Focus::Diff,
            Focus::Diff => Focus::Stage,
            Focus::Stage => Focus::Status,
        });
    }

    fn switch_focus(&mut self, f: Focus) {
        if self.focus != f {
            self.focus = f;

            match self.focus {
                Focus::Status => {
                    self.diff_target = DiffTarget::WorkingDir;
                    self.index_wd.focus(true);
                    self.index.focus(false);
                    self.diff.focus(false);
                }
                Focus::Stage => {
                    self.diff_target = DiffTarget::Stage;
                    self.index.focus(true);
                    self.index_wd.focus(false);
                    self.diff.focus(false);
                }
                Focus::Diff => {
                    self.index.focus(false);
                    self.index_wd.focus(false);
                    self.diff.focus(true);
                }
            };
        }
    }

    fn index_add_remove(&mut self) {
        if self.index_wd.focused() {
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
