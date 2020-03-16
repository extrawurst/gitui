use crossterm::event::{Event, KeyCode};
use git2::Repository;
use std::cmp;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, SelectableList, Widget},
    Frame,
};

#[derive(Default)]
pub struct App {
    status_items: Vec<String>,
    status_select: Option<usize>,
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

        // println!("state: {:?}",repo.state());
        // println!("path: {:?}",repo.path());

        if repo.is_bare() {
            panic!("bare repo")
        }

        let status = repo.statuses(None).unwrap();

        self.status_items = status
            .iter()
            .map(|e| e.path().unwrap().to_string())
            .collect();

        self.status_select = if self.status_items.len() > 0 {
            Some(0)
        } else {
            None
        };
    }

    ///
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(f.size());

        draw_list(
            f,
            chunks[0],
            "Status".to_string(),
            self.status_items.as_slice(),
            self.status_select,
        );

        Block::default()
            .title("Block 2")
            .borders(Borders::ALL)
            .render(f, chunks[1]);
    }

    ///
    pub fn event(&mut self, ev: Event) {
        if ev == Event::Key(KeyCode::Esc.into()) {
            self.do_quit = true;
        }

        if ev == Event::Key(KeyCode::Up.into()) {
            self.input(-1);
        }
        if ev == Event::Key(KeyCode::Down.into()) {
            self.input(1);
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
        .highlight_style(Style::default().modifier(Modifier::ITALIC))
        .highlight_symbol(">")
        .render(f, r);
}
