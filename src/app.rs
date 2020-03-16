use crossterm::event::{Event, KeyCode};
use git2::{DiffFormat, Repository, Status};
use std::cmp;
use std::path::Path;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, SelectableList, Text, Widget},
    Frame,
};

#[derive(Copy, Clone,PartialEq)]
pub enum DiffLineType {
    None,
    Header,
    Add,
    Delete,
}

impl Default for DiffLineType {
    fn default() -> Self {
        DiffLineType::None
    }
}

#[derive(Default,PartialEq)]
pub struct DiffLine {
    content: String,
    line_type: DiffLineType,
}

#[derive(Default,PartialEq)]
pub struct Diff(Vec<DiffLine>);

#[derive(Default)]
pub struct App {
    status_items: Vec<String>,
    index_items: Vec<String>,
    status_select: Option<usize>,
    diff: Diff,
    offset: u16,
    do_quit: bool,
}

impl App {
    ///
    pub fn is_quit(&self) -> bool {
        self.do_quit
    }
}

impl App {
    //
    pub fn fetch_status(&mut self) {
        let repo = match Repository::init("./") {
            Ok(repo) => repo,
            Err(e) => panic!("failed to init: {}", e),
        };

        if repo.is_bare() {
            panic!("bare repo")
        }

        let statuses = repo.statuses(None).unwrap();

        self.status_items = Vec::new();
        self.index_items = Vec::new();

        for e in statuses.iter() {
            let status: Status = e.status();
            if status.is_ignored() {
                continue;
            }

            if status.is_index_new() || status.is_index_modified() {
                self.index_items
                    .push(format!("{} ({:?})", e.path().unwrap().to_string(), status))
            }

            if status.is_wt_new() || status.is_wt_modified() {
                self.status_items.push(e.path().unwrap().to_string())
            }
        }

        self.status_select = if self.status_items.len() > 0 {
            Some(0)
        } else {
            None
        };

        self.update_diff();
    }

    ///
    fn update_diff(&mut self) {
        let new_diff=match self.status_select {
            Some(i) => get_diff(Path::new(self.status_items[i].as_str())),
            None => Diff::default(),
        };

        if new_diff != self.diff {
            self.diff = new_diff;
            self.offset = 0;
        }
    }

    ///
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(f.size());

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[0]);

        draw_list(
            f,
            left_chunks[0],
            "Status".to_string(),
            self.status_items.as_slice(),
            self.status_select,
        );

        draw_list(
            f,
            left_chunks[1],
            "Index".to_string(),
            self.index_items.as_slice(),
            None,
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
                        Style::default().fg(Color::White).bg(Color::Red),
                    ),
                    DiffLineType::Add => Text::Styled(
                        content.into(),
                        Style::default().fg(Color::White).bg(Color::Green),
                    ),
                    _ => Text::Raw(content.into()),
                }
            })
            .collect::<Vec<_>>();

        Paragraph::new(txt.iter())
            .block(Block::default().title("Diff").borders(Borders::ALL))
            .alignment(Alignment::Left)
            .scroll(self.offset)
            .render(f, chunks[1]);
    }

    ///
    pub fn event(&mut self, ev: Event) {
        if ev == Event::Key(KeyCode::Esc.into()) || ev == Event::Key(KeyCode::Char('q').into()) {
            self.do_quit = true;
        }

        if ev == Event::Key(KeyCode::Up.into()) {
            self.input(-1);
        }
        if ev == Event::Key(KeyCode::Down.into()) {
            self.input(1);
        }

        if ev == Event::Key(KeyCode::PageDown.into()) {
            self.offset += 1;
        }
        if ev == Event::Key(KeyCode::PageUp.into()) {
            if self.offset > 0 {
                self.offset -= 1;
            }
        }

        if ev == Event::Key(KeyCode::Enter.into()) {
            // self.index_add();
        }
    }

    fn input(&mut self, delta: i32) {
        let items_len = self.status_items.len();
        if items_len > 0 {
            if let Some(i) = self.status_select {
                let mut i = i as i32;

                i = cmp::min(i + delta, (items_len - 1) as i32);
                i = cmp::max(i, 0);

                self.status_select = Some(i as usize);
            }
        }

        self.update_diff();
    }
}

fn draw_list<B: Backend, T: AsRef<str>>(
    f: &mut Frame<B>,
    r: Rect,
    title: String,
    items: &[T],
    select: Option<usize>,
) {
    SelectableList::default()
        .block(Block::default().title(title.as_str()).borders(Borders::ALL))
        .items(items)
        .select(select)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().modifier(Modifier::BOLD))
        .highlight_symbol(">")
        .render(f, r);
}

///
fn get_diff(p: &Path) -> Diff {
    let repo = Repository::init("./").unwrap();

    if repo.is_bare() {
        panic!("bare repo")
    }

    let diff = repo.diff_index_to_workdir(None, None).unwrap();

    let mut res = Vec::new();

    diff.print(DiffFormat::Patch, |delta, _hunk, line| {
        if p != delta.old_file().path().unwrap() {
            return true;
        }
        if p != delta.new_file().path().unwrap() {
            return true;
        }

        let line_type = match line.origin() {
            'H' => DiffLineType::Header,
            '<' | '-' => DiffLineType::Delete,
            '>' | '+' => DiffLineType::Add,
            _ => DiffLineType::None,
        };

        let diff_line = DiffLine {
            content: String::from_utf8_lossy(line.content()).to_string(),
            line_type,
        };

        res.push(diff_line);
        true
    })
    .unwrap();

    Diff(res)
}
