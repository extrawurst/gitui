use super::{
    filetree::{FileTreeItem, FileTreeItemKind},
    statustree::{MoveSelection, StatusTree},
    CommandBlocking, DrawableComponent,
};
use crate::{
    components::{CommandInfo, Component},
    keys,
    queue::{InternalEvent, NeedsUpdate, Queue, ResetItem},
    strings, ui,
    ui::style::Theme,
};
use asyncgit::{hash, sync, StatusItem, StatusItemType, CWD};
use crossterm::event::Event;
use std::{borrow::Cow, convert::From, path::Path};
use strings::commands;
use tui::{backend::Backend, layout::Rect, widgets::Text, Frame};

///
pub struct ChangesComponent {
    title: String,
    tree: StatusTree,
    current_hash: u64,
    focused: bool,
    show_selection: bool,
    is_working_dir: bool,
    queue: Queue,
    theme: Theme,
}

impl ChangesComponent {
    ///
    pub fn new(
        title: &str,
        focus: bool,
        is_working_dir: bool,
        queue: Queue,
        theme: &Theme,
    ) -> Self {
        Self {
            title: title.to_string(),
            tree: StatusTree::default(),
            current_hash: 0,
            focused: focus,
            show_selection: focus,
            is_working_dir,
            queue,
            theme: *theme,
        }
    }

    ///
    pub fn update(&mut self, list: &[StatusItem]) {
        let new_hash = hash(list);
        if self.current_hash != new_hash {
            self.tree.update(list);
            self.current_hash = new_hash;
        }
    }

    ///
    pub fn selection(&self) -> Option<FileTreeItem> {
        self.tree.selected_item()
    }

    ///
    pub fn focus_select(&mut self, focus: bool) {
        self.focus(focus);
        self.show_selection = focus;
    }

    /// returns true if list is empty
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
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
            self.queue
                .borrow_mut()
                .push_back(InternalEvent::Update(NeedsUpdate::DIFF));
        }

        changed
    }

    fn index_add_remove(&mut self) -> bool {
        if let Some(tree_item) = self.selection() {
            if self.is_working_dir {
                if let FileTreeItemKind::File(i) = tree_item.kind {
                    if let Some(status) = i.status {
                        let path = Path::new(i.path.as_str());
                        return match status {
                            StatusItemType::Deleted => {
                                sync::stage_addremoved(CWD, path)
                                    .is_ok()
                            }
                            _ => sync::stage_add_file(CWD, path)
                                .is_ok(),
                        };
                    }
                } else {
                    //TODO: check if we can handle the one file case with it aswell
                    return sync::stage_add_all(
                        CWD,
                        tree_item.info.full_path.as_str(),
                    )
                    .is_ok();
                }
            } else {
                let path =
                    Path::new(tree_item.info.full_path.as_str());
                sync::reset_stage(CWD, path).unwrap();
                return true;
            }
        }

        false
    }

    fn dispatch_reset_workdir(&mut self) -> bool {
        if let Some(tree_item) = self.selection() {
            let is_folder =
                matches!(tree_item.kind, FileTreeItemKind::Path(_));
            self.queue.borrow_mut().push_back(
                InternalEvent::ConfirmResetItem(ResetItem {
                    path: tree_item.info.full_path,
                    is_folder,
                }),
            );

            return true;
        }
        false
    }

    fn item_to_text(
        item: &FileTreeItem,
        width: u16,
        selected: bool,
        theme: Theme,
    ) -> Option<Text> {
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
                    .unwrap()
                    .to_str()
                    .unwrap();

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

                let status = status_item
                    .status
                    .unwrap_or(StatusItemType::Modified);

                Some(Text::Styled(
                    Cow::from(txt),
                    theme.item(status, selected),
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

    fn item_status_char(item_type: Option<StatusItemType>) -> char {
        if let Some(item_type) = item_type {
            match item_type {
                StatusItemType::Modified => 'M',
                StatusItemType::New => '+',
                StatusItemType::Deleted => '-',
                StatusItemType::Renamed => 'R',
                _ => ' ',
            }
        } else {
            ' '
        }
    }
}

impl DrawableComponent for ChangesComponent {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, r: Rect) {
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
                        self.theme,
                    )
                },
            );

        ui::draw_list(
            f,
            r,
            &self.title.to_string(),
            items,
            self.tree.selection.map(|idx| idx - selection_offset),
            self.focused,
            self.theme,
        );
    }
}

impl Component for ChangesComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        let some_selection = self.selection().is_some();

        if self.is_working_dir {
            out.push(CommandInfo::new(
                commands::STAGE_ITEM,
                some_selection,
                self.focused,
            ));
            out.push(CommandInfo::new(
                commands::RESET_ITEM,
                some_selection,
                self.focused,
            ));
        } else {
            out.push(CommandInfo::new(
                commands::UNSTAGE_ITEM,
                some_selection,
                self.focused,
            ));
            out.push(
                CommandInfo::new(
                    commands::COMMIT_OPEN,
                    !self.is_empty(),
                    self.focused || force_all,
                )
                .order(-1),
            );
        }

        out.push(CommandInfo::new(
            commands::NAVIGATE_TREE,
            !self.is_empty(),
            self.focused,
        ));

        CommandBlocking::PassingOn
    }

    fn event(&mut self, ev: Event) -> bool {
        if self.focused {
            if let Event::Key(e) = ev {
                return match e {
                    keys::OPEN_COMMIT
                        if !self.is_working_dir
                            && !self.is_empty() =>
                    {
                        self.queue
                            .borrow_mut()
                            .push_back(InternalEvent::OpenCommit);
                        true
                    }
                    keys::STATUS_STAGE_FILE => {
                        if self.index_add_remove() {
                            self.queue.borrow_mut().push_back(
                                InternalEvent::Update(
                                    NeedsUpdate::ALL,
                                ),
                            );
                        }
                        true
                    }
                    keys::STATUS_RESET_FILE
                        if self.is_working_dir =>
                    {
                        self.dispatch_reset_workdir()
                    }
                    keys::MOVE_DOWN => {
                        self.move_selection(MoveSelection::Down)
                    }
                    keys::MOVE_UP => {
                        self.move_selection(MoveSelection::Up)
                    }
                    keys::HOME | keys::SHIFT_UP => {
                        self.move_selection(MoveSelection::Home)
                    }
                    keys::END | keys::SHIFT_DOWN => {
                        self.move_selection(MoveSelection::End)
                    }
                    keys::MOVE_LEFT => {
                        self.move_selection(MoveSelection::Left)
                    }
                    keys::MOVE_RIGHT => {
                        self.move_selection(MoveSelection::Right)
                    }
                    _ => false,
                };
            }
        }

        false
    }

    fn focused(&self) -> bool {
        self.focused
    }
    fn focus(&mut self, focus: bool) {
        self.focused = focus
    }
}
