use crate::{
    components::{CommandInfo, Component},
    strings, ui,
};
use asyncgit::{hash, StatusItem, StatusItemType};
use crossterm::event::{Event, KeyCode};
use std::{borrow::Cow, cmp};
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Text,
    Frame,
};

///
pub struct IndexComponent {
    title: String,
    items: Vec<StatusItem>,
    selection: Option<usize>,
    focused: bool,
    show_selection: bool,
}

impl IndexComponent {
    ///
    pub fn new(title: &str, focus: bool) -> Self {
        Self {
            title: title.to_string(),
            items: Vec::new(),

            selection: None,
            focused: focus,
            show_selection: focus,
        }
    }

    ///
    pub fn update(&mut self, list: &[StatusItem]) {
        if hash(&self.items) != hash(list) {
            self.items = list.to_owned();

            let old_selection = self.selection.unwrap_or_default();
            self.selection = if self.items.is_empty() {
                None
            } else {
                Some(cmp::min(old_selection, self.items.len() - 1))
            };
        }
    }

    ///
    pub fn selection(&self) -> Option<StatusItem> {
        match self.selection {
            None => None,
            Some(i) => Some(self.items[i].clone()),
        }
    }

    ///
    pub fn focus_select(&mut self, focus: bool) {
        self.focus(focus);
        self.show_selection = focus;
    }

    ///
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
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
        let item_to_text = |idx: usize, i: &StatusItem| -> Text {
            let selected = self.show_selection
                && self.selection.map_or(false, |e| e == idx);
            let txt = if selected {
                format!("> {}", i.path)
            } else {
                format!("  {}", i.path)
            };
            let mut style = Style::default().fg(
                match i.status.unwrap_or(StatusItemType::Modified) {
                    StatusItemType::Modified => Color::LightYellow,
                    StatusItemType::New => Color::LightGreen,
                    StatusItemType::Deleted => Color::LightRed,
                    _ => Color::White,
                },
            );
            if selected {
                style = style.modifier(Modifier::BOLD); //.fg(Color::White);
            }

            Text::Styled(Cow::from(txt), style)
        };

        ui::draw_list(
            f,
            r,
            &self.title.to_string(),
            self.items
                .iter()
                .enumerate()
                .map(|(idx, e)| item_to_text(idx, e)),
            if self.show_selection {
                self.selection
            } else {
                None
            },
            self.focused,
        );
    }

    fn commands(&self) -> Vec<CommandInfo> {
        vec![CommandInfo::new(
            strings::CMD_SCROLL,
            self.items.len() > 1,
            self.focused,
        )]
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
