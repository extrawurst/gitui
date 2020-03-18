use crate::{
    components::{
        CommandInfo, CommitComponent, Component, IndexComponent,
    },
    git_status::StatusLists,
    git_utils::{self, Diff, DiffLine, DiffLineType},
    tui_utils,
};
use crossterm::event::{Event, KeyCode, MouseEvent};
use git2::IndexAddOption;
use itertools::Itertools;
use std::{borrow::Cow, path::Path};
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Tabs, Text, Widget},
    Frame,
};

#[derive(Default)]
pub struct App {
    status: StatusLists,
    diff: Diff,
    offset: u16,
    do_quit: bool,
    commit: CommitComponent,
    index: IndexComponent,
    // index_wt: IndexComponent,
}

impl App {
    ///
    pub fn is_quit(&self) -> bool {
        self.do_quit
    }
}

impl App {
    ///
    fn fetch_status(&mut self) {
        let new_status = StatusLists::new();

        if self.status != new_status {
            self.status = new_status;
        }
    }

    ///
    fn update_diff(&mut self) {
        let new_diff = match self.index.selection() {
            Some(i) => {
                git_utils::get_diff(Path::new(i.path.as_str()))
            }
            None => Diff::default(),
        };

        if new_diff != self.diff {
            self.diff = new_diff;
            self.offset = 0;
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
            .titles(&["Status", "Log", "Stash", "Misc"])
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

        self.index.draw(f, left_chunks[0]);

        tui_utils::draw_list(
            f,
            left_chunks[1],
            "Index [i]".to_string(),
            self.status.index_items_pathlist().as_slice(),
            None,
            false,
        );

        let txt = self
            .diff
            .0
            .iter()
            .map(|e: &DiffLine| {
                let content = e.content.clone();
                match e.line_type {
                    DiffLineType::Delete => Text::Styled(
                        content.into(),
                        Style::default()
                            .fg(Color::Red)
                            .bg(Color::Black),
                    ),
                    DiffLineType::Add => Text::Styled(
                        content.into(),
                        Style::default()
                            .fg(Color::Green)
                            .bg(Color::Black),
                    ),
                    DiffLineType::Header => Text::Styled(
                        content.into(),
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Gray)
                            .modifier(Modifier::BOLD),
                    ),
                    _ => Text::Raw(content.into()),
                }
            })
            .collect::<Vec<_>>();

        Paragraph::new(txt.iter())
            .block(
                Block::default()
                    .title("Diff [d]")
                    .borders(Borders::ALL),
            )
            .alignment(Alignment::Left)
            .scroll(self.offset)
            .render(f, chunks[1]);

        let mut cmds = self.commit.commands();
        cmds.extend(self.index.commands());
        cmds.extend(self.commands());

        self.draw_commands(f, chunks_main[2], cmds);

        self.commit.draw(f, f.size());
    }

    fn commands(&self) -> Vec<CommandInfo> {
        if !self.commit.is_visible() {
            vec![CommandInfo {
                name: "Quit [esc,q]".to_string(),
                enabled: true,
            }]
        } else {
            Vec::new()
        }
    }

    ///
    pub fn event(&mut self, ev: Event) {
        if self.commit.event(ev) {
            return;
        }

        if self.index.event(ev) {
            return;
        }

        if !self.commit.is_visible() {
            if ev == Event::Key(KeyCode::Esc.into())
                || ev == Event::Key(KeyCode::Char('q').into())
            {
                self.do_quit = true;
            }

            if ev == Event::Key(KeyCode::PageDown.into()) {
                self.scroll(true);
            }
            if ev == Event::Key(KeyCode::PageUp.into()) {
                self.scroll(false);
            }
            if let Event::Mouse(MouseEvent::ScrollDown(_, _, _)) = ev
            {
                self.scroll(true);
            }
            if let Event::Mouse(MouseEvent::ScrollUp(_, _, _)) = ev {
                self.scroll(false);
            }

            if ev == Event::Key(KeyCode::Enter.into()) {
                self.index_add();
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
        self.fetch_status();
        self.index.update();
        self.update_diff();
    }

    fn index_add(&mut self) {
        if let Some(i) = self.index.selection() {
            let repo = git_utils::repo();

            let mut index = repo.index().unwrap();

            let path = Path::new(i.path.as_str());

            let cb = &mut |p: &Path, _matched_spec: &[u8]| -> i32 {
                if p == path {
                    0
                } else {
                    1
                }
            };

            if let Ok(_) = index.add_all(
                path,
                IndexAddOption::DISABLE_PATHSPEC_MATCH
                    | IndexAddOption::CHECK_PATHSPEC,
                Some(cb as &mut git2::IndexMatchedPath),
            ) {
                index.write().unwrap();
                self.update();
            }
        }
    }

    fn scroll(&mut self, inc: bool) {
        if inc {
            self.offset =
                self.offset.checked_add(1).unwrap_or(self.offset);
        } else {
            self.offset = self.offset.checked_sub(1).unwrap_or(0);
        }
    }
}
