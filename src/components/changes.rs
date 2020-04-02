use super::{CommandBlocking, DrawableComponent, EventUpdate};
use crate::{
    components::{CommandInfo, Component},
    keys, strings, ui,
};
use asyncgit::{hash, sync, StatusItem, StatusItemType, CWD};
use crossterm::event::Event;
use std::{borrow::Cow, cmp, path::Path};
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Text,
    Frame,
};

///
pub struct ChangesComponent {
    title: String,
    items: Vec<StatusItem>,
    selection: Option<usize>,
    focused: bool,
    show_selection: bool,
    is_working_dir: bool,
}

impl ChangesComponent {
    ///
    pub fn new(
        title: &str,
        focus: bool,
        is_working_dir: bool,
    ) -> Self {
        Self {
            title: title.to_string(),
            items: Vec::new(),

            selection: None,
            focused: focus,
            show_selection: focus,
            is_working_dir,
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

    fn index_add_remove(&mut self) -> bool {
        if let Some(i) = self.selection() {
            if self.is_working_dir {
                let path = Path::new(i.path.as_str());

                return sync::stage_add(CWD, path);
            } else {
                let path = Path::new(i.path.as_str());

                return sync::reset_stage(CWD, path);
            }
        }

        false
    }

    fn index_reset(&mut self) -> bool {
        if let Some(i) = self.selection() {
            let path = Path::new(i.path.as_str());

            if sync::reset_workdir(CWD, path) {
                return true;
            }
        }
        false
    }
}

impl DrawableComponent for ChangesComponent {
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
}

impl Component for ChangesComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
    ) -> CommandBlocking {
        let some_selection = self.selection().is_some();
        if self.is_working_dir {
            out.push(CommandInfo::new(
                strings::CMD_STATUS_STAGE,
                some_selection,
                self.focused,
            ));
            out.push(CommandInfo::new(
                strings::CMD_STATUS_RESET,
                some_selection,
                self.focused,
            ));
        } else {
            out.push(CommandInfo::new(
                strings::CMD_STATUS_UNSTAGE,
                some_selection,
                self.focused,
            ));
        }

        out.push(CommandInfo::new(
            strings::CMD_SCROLL,
            self.items.len() > 1,
            self.focused,
        ));

        CommandBlocking::PassingOn
    }

    fn event(&mut self, ev: Event) -> Option<EventUpdate> {
        if self.focused {
            if let Event::Key(e) = ev {
                return match e {
                    keys::STATUS_STAGE_FILE => {
                        if self.index_add_remove() {
                            Some(EventUpdate::All)
                        } else {
                            Some(EventUpdate::None)
                        }
                    }
                    keys::STATUS_RESET_FILE => {
                        if self.index_reset() {
                            Some(EventUpdate::All)
                        } else {
                            Some(EventUpdate::None)
                        }
                    }
                    keys::MOVE_DOWN => {
                        self.move_selection(1);
                        Some(EventUpdate::Diff)
                    }
                    keys::MOVE_UP => {
                        self.move_selection(-1);
                        Some(EventUpdate::Diff)
                    }
                    _ => None,
                };
            }
        }

        None
    }

    fn focused(&self) -> bool {
        self.focused
    }
    fn focus(&mut self, focus: bool) {
        self.focused = focus
    }
}
