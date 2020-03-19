use crate::components::CommandInfo;
use crate::components::Component;
use crate::{
    git_status::{self, StatusItem},
    tui_utils,
};
use crossterm::event::{Event, KeyCode};
use git2::StatusShow;
use std::cmp;
use tui::{backend::Backend, layout::Rect, Frame};

///
pub struct IndexComponent {
    title: String,
    items: Vec<StatusItem>,
    index_type: StatusShow,
    selection: Option<usize>,
    focused: bool,
}

impl IndexComponent {
    ///
    pub fn new(
        title: &str,
        index_type: StatusShow,
        focus: bool,
    ) -> Self {
        Self {
            title: title.to_string(),
            items: Vec::new(),
            index_type,
            selection: None,
            focused: focus,
        }
    }
    ///
    pub fn update(&mut self) {
        let new_status = git_status::get_index(self.index_type);

        if self.items != new_status {
            self.items = new_status;

            self.selection =
                if self.items.len() > 0 { Some(0) } else { None };
        }
    }

    ///
    pub fn selection(&self) -> Option<StatusItem> {
        match self.selection {
            None => None,
            Some(i) => Some(self.items[i].clone()),
        }
    }

    fn move_selection(&mut self, delta: i32) {
        let items_len = self.items.len();
        if items_len > 0 {
            if let Some(i) = self.selection {
                let mut i = i as i32;

                i = cmp::min(i + delta, (items_len - 1) as i32);
                i = cmp::max(i, 0);

                self.selection = Some(i as usize);
            }
        }
    }
}

impl Component for IndexComponent {
    fn draw<B: Backend>(&self, f: &mut Frame<B>, r: Rect) {
        tui_utils::draw_list(
            f,
            r,
            self.title.to_string(),
            self.items
                .iter()
                .map(|e| e.path.clone())
                .collect::<Vec<_>>()
                .as_slice(),
            if self.focused { self.selection } else { None },
            self.focused,
        );
    }

    fn commands(&self) -> Vec<CommandInfo> {
        if self.focused {
            return vec![CommandInfo {
                name: "Scroll [↑↓]".to_string(),
                enabled: self.items.len() > 0,
            }];
        }

        Vec::new()
    }

    fn event(&mut self, ev: Event) -> bool {
        if self.focused {
            if let Event::Key(e) = ev {
                return match e.code {
                    KeyCode::Down => {
                        self.move_selection(1);
                        true
                    }
                    KeyCode::Up => {
                        self.move_selection(-1);
                        true
                    }
                    _ => false,
                };
            }
        }

        false
    }

    ///
    fn focused(&self) -> bool {
        self.focused
    }
    ///
    fn focus(&mut self, focus: bool) {
        self.focused = focus
    }
}
