use crossterm::event::{Event, KeyCode};
use git2::{DiffFormat, Repository};
use std::cmp;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, SelectableList, Widget, Paragraph, Text},
    Frame,
};

#[derive(Default)]
pub struct App {
    status_items: Vec<String>,
    status_select: Option<usize>,
    diff: String,
    offset:u16,
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

        self.diff = self.get_diff();
    }

    ///
    pub fn get_diff(&mut self) -> String {
        let repo = Repository::init("./").unwrap();

        if repo.is_bare() {
            panic!("bare repo")
        }

        let diff = repo.diff_index_to_workdir(None, None).unwrap();

        let mut res = String::new();

        diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
            let content = String::from_utf8_lossy(line.content());
            res.push_str(content.chars().as_str());
            true
        })
        .unwrap();

        res
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

        Paragraph::new([Text::raw(self.diff.clone())].iter())
            .block(Block::default().title("Diff").borders(Borders::ALL))
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left)
            .scroll(self.offset)
            .render(f, chunks[1]);

        // Block::default()
        //     .title("Diff")
        //     .borders(Borders::ALL)
        //     .render(f, chunks[1]);
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

        if ev == Event::Key(KeyCode::PageDown.into()) {
            self.offset+=1;
        }
        if ev == Event::Key(KeyCode::PageUp.into()) {
            if self.offset>0{
                self.offset-=1;
            }
        }

        if ev == Event::Key(KeyCode::Enter.into()) {
            // self.index_add();
        }
    }

    // fn index_add(&mut self) {
    //     let repo = Repository::init("./").unwrap();

    //     let status = repo.statuses(None).unwrap();

    //     let index = repo.index().unwrap();
    //     index.add(entry)

    //     self.status_items = status
    //         .iter()
    //         .map(|e| e.path().unwrap().to_string())
    //         .collect();

    //     self.status_select = if self.status_items.len() > 0 {
    //         Some(0)
    //     } else {
    //         None
    //     };
    // }

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
        .highlight_style(Style::default().modifier(Modifier::BOLD))
        .highlight_symbol(">")
        .render(f, r);
}
