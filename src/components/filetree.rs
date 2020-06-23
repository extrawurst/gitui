use super::{
    utils::{
        filetree::{FileTreeItem, FileTreeItemKind},
        statustree::{MoveSelection, StatusTree},
    },
    CommandBlocking, DrawableComponent,
};
use crate::{
    components::{CommandInfo, Component},
    keys,
    queue::{InternalEvent, NeedsUpdate, Queue},
    strings, ui,
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{hash, StatusItem, StatusItemType};
use crossterm::event::Event;
use std::{borrow::Cow, convert::From, path::Path};
use strings::{commands, order};
use tui::{backend::Backend, layout::Rect, widgets::Text, Frame};

///
pub struct FileTreeComponent {
    title: String,
    tree: StatusTree,
    current_hash: u64,
    focused: bool,
    show_selection: bool,
    queue: Option<Queue>,
    theme: SharedTheme,
}

impl FileTreeComponent {
    ///
    pub fn new(
        title: &str,
        focus: bool,
        queue: Option<Queue>,
        theme: SharedTheme,
    ) -> Self {
        Self {
            title: title.to_string(),
            tree: StatusTree::default(),
            current_hash: 0,
            focused: focus,
            show_selection: focus,
            queue,
            theme,
        }
    }

    ///
    pub fn update(&mut self, list: &[StatusItem]) -> Result<()> {
        let new_hash = hash(list);
        if self.current_hash != new_hash {
            self.tree.update(list)?;
            self.current_hash = new_hash;
        }

        Ok(())
    }

    ///
    pub fn selection(&self) -> Option<FileTreeItem> {
        self.tree.selected_item()
    }

    ///
    pub fn selection_file(&self) -> Option<StatusItem> {
        self.tree.selected_item().and_then(|f| {
            if let FileTreeItemKind::File(f) = f.kind {
                Some(f)
            } else {
                None
            }
        })
    }

    ///
    pub fn show_selection(&mut self, show: bool) {
        self.show_selection = show;
    }

    /// returns true if list is empty
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    ///
    pub const fn file_count(&self) -> usize {
        self.tree.tree.file_count()
    }

    ///
    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    ///
    pub fn clear(&mut self) -> Result<()> {
        self.current_hash = 0;
        self.tree.update(&[])
    }

    ///
    pub fn is_file_seleted(&self) -> bool {
        if let Some(item) = self.tree.selected_item() {
            match item.kind {
                FileTreeItemKind::File(_) => true,
                _ => false,
            }
        } else {
            false
        }
    }

    fn move_selection(&mut self, dir: MoveSelection) -> bool {
        let changed = self.tree.move_selection(dir);

        if changed {
            if let Some(ref queue) = self.queue {
                queue.borrow_mut().push_back(InternalEvent::Update(
                    NeedsUpdate::DIFF,
                ));
            }
        }

        changed
    }

    fn item_to_text<'a>(
        item: &FileTreeItem,
        width: u16,
        selected: bool,
        theme: &'a SharedTheme,
    ) -> Option<Text<'a>> {
        let indent_str = if item.info.indent == 0 {
            String::from("")
        } else {
            format!("{:w$}", " ", w = (item.info.indent as usize) * 2)
        };

        if !item.info.visible {
            return None;
        }

        match &item.kind {
            FileTreeItemKind::File(status_item) => {
                let status_char =
                    Self::item_status_char(status_item.status);
                let file = Path::new(&status_item.path)
                    .file_name()
                    .and_then(std::ffi::OsStr::to_str)
                    .expect("invalid path.");

                let txt = if selected {
                    format!(
                        "{} {}{:w$}",
                        status_char,
                        indent_str,
                        file,
                        w = width as usize
                    )
                } else {
                    format!("{} {}{}", status_char, indent_str, file)
                };

                Some(Text::Styled(
                    Cow::from(txt),
                    theme.item(status_item.status, selected),
                ))
            }

            FileTreeItemKind::Path(path_collapsed) => {
                let collapse_char =
                    if path_collapsed.0 { '▸' } else { '▾' };

                let txt = if selected {
                    format!(
                        "  {}{}{:w$}",
                        indent_str,
                        collapse_char,
                        item.info.path,
                        w = width as usize
                    )
                } else {
                    format!(
                        "  {}{}{}",
                        indent_str, collapse_char, item.info.path,
                    )
                };

                Some(Text::Styled(
                    Cow::from(txt),
                    theme.text(true, selected),
                ))
            }
        }
    }

    fn item_status_char(item_type: StatusItemType) -> char {
        match item_type {
            StatusItemType::Modified => 'M',
            StatusItemType::New => '+',
            StatusItemType::Deleted => '-',
            StatusItemType::Renamed => 'R',
            _ => ' ',
        }
    }
}

impl DrawableComponent for FileTreeComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        r: Rect,
    ) -> Result<()> {
        let selection_offset =
            self.tree.tree.items().iter().enumerate().fold(
                0,
                |acc, (idx, e)| {
                    let visible = e.info.visible;
                    let index_above_select =
                        idx < self.tree.selection.unwrap_or(0);

                    if !visible && index_above_select {
                        acc + 1
                    } else {
                        acc
                    }
                },
            );

        let items =
            self.tree.tree.items().iter().enumerate().filter_map(
                |(idx, e)| {
                    Self::item_to_text(
                        e,
                        r.width,
                        self.show_selection
                            && self
                                .tree
                                .selection
                                .map_or(false, |e| e == idx),
                        &self.theme,
                    )
                },
            );

        ui::draw_list(
            f,
            r,
            self.title.as_str(),
            items,
            self.tree.selection.map(|idx| idx - selection_offset),
            self.focused,
            &self.theme,
        );

        Ok(())
    }
}

impl Component for FileTreeComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        out.push(
            CommandInfo::new(
                commands::NAVIGATE_TREE,
                !self.is_empty(),
                self.focused || force_all,
            )
            .order(order::NAV),
        );

        CommandBlocking::PassingOn
    }

    fn event(&mut self, ev: Event) -> Result<bool> {
        if self.focused {
            if let Event::Key(e) = ev {
                return match e {
                    keys::MOVE_DOWN => {
                        Ok(self.move_selection(MoveSelection::Down))
                    }
                    keys::MOVE_UP => {
                        Ok(self.move_selection(MoveSelection::Up))
                    }
                    keys::HOME | keys::SHIFT_UP => {
                        Ok(self.move_selection(MoveSelection::Home))
                    }
                    keys::END | keys::SHIFT_DOWN => {
                        Ok(self.move_selection(MoveSelection::End))
                    }
                    keys::MOVE_LEFT => {
                        Ok(self.move_selection(MoveSelection::Left))
                    }
                    keys::MOVE_RIGHT => {
                        Ok(self.move_selection(MoveSelection::Right))
                    }
                    _ => Ok(false),
                };
            }
        }

        Ok(false)
    }

    fn focused(&self) -> bool {
        self.focused
    }
    fn focus(&mut self, focus: bool) {
        self.focused = focus;
    }
}
