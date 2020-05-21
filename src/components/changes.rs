use super::{
    filetree::FileTreeComponent,
    utils::filetree::{FileTreeItem, FileTreeItemKind},
    CommandBlocking, DrawableComponent,
};
use crate::{
    components::{CommandInfo, Component},
    keys,
    queue::{InternalEvent, NeedsUpdate, Queue, ResetItem},
    strings,
    ui::style::Theme,
};
use asyncgit::{sync, StatusItem, StatusItemType, CWD};
use crossterm::event::Event;
use std::path::Path;
use strings::commands;
use tui::{backend::Backend, layout::Rect, Frame};

///
pub struct ChangesComponent {
    files: FileTreeComponent,
    is_working_dir: bool,
    queue: Queue,
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
            files: FileTreeComponent::new(
                title,
                focus,
                queue.clone(),
                theme,
            ),
            is_working_dir,
            queue,
        }
    }

    ///
    pub fn update(&mut self, list: &[StatusItem]) {
        self.files.update(list)
    }

    ///
    pub fn selection(&self) -> Option<FileTreeItem> {
        self.files.selection()
    }

    ///
    pub fn focus_select(&mut self, focus: bool) {
        self.files.focus_select(focus)
    }

    /// returns true if list is empty
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    ///
    pub fn is_file_seleted(&self) -> bool {
        self.files.is_file_seleted()
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
}

impl DrawableComponent for ChangesComponent {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, r: Rect) {
        self.files.draw(f, r)
    }
}

impl Component for ChangesComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        self.files.commands(out, force_all);

        let some_selection = self.selection().is_some();

        if self.is_working_dir {
            out.push(CommandInfo::new(
                commands::STAGE_ITEM,
                some_selection,
                self.focused(),
            ));
            out.push(CommandInfo::new(
                commands::RESET_ITEM,
                some_selection,
                self.focused(),
            ));
        } else {
            out.push(CommandInfo::new(
                commands::UNSTAGE_ITEM,
                some_selection,
                self.focused(),
            ));
            out.push(
                CommandInfo::new(
                    commands::COMMIT_OPEN,
                    !self.is_empty(),
                    self.focused() || force_all,
                )
                .order(-1),
            );
        }

        CommandBlocking::PassingOn
    }

    fn event(&mut self, ev: Event) -> bool {
        if self.files.event(ev) {
            return true;
        }

        if self.focused() {
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
                    _ => false,
                };
            }
        }

        false
    }

    fn focused(&self) -> bool {
        self.files.focused()
    }
    fn focus(&mut self, focus: bool) {
        self.files.focus(focus)
    }
}
