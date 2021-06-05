use super::{
    filetree::FileTreeComponent,
    utils::filetree::{FileTreeItem, FileTreeItemKind},
    CommandBlocking, DrawableComponent,
};
use crate::{
    components::{CommandInfo, Component, EventState},
    keys::SharedKeyConfig,
    queue::{Action, InternalEvent, NeedsUpdate, Queue, ResetItem},
    strings, try_or_popup,
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{sync, StatusItem, StatusItemType, CWD};
use crossterm::event::Event;
use std::path::Path;
use tui::{backend::Backend, layout::Rect, Frame};

///
pub struct ChangesComponent {
    files: FileTreeComponent,
    is_working_dir: bool,
    queue: Queue,
    key_config: SharedKeyConfig,
}

impl ChangesComponent {
    ///
    pub fn new(
        title: &str,
        focus: bool,
        is_working_dir: bool,
        queue: Queue,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            files: FileTreeComponent::new(
                title,
                focus,
                Some(queue.clone()),
                theme,
                key_config.clone(),
            ),
            is_working_dir,
            queue,
            key_config,
        }
    }

    ///
    pub fn set_items(&mut self, list: &[StatusItem]) -> Result<()> {
        self.files.update(list)?;
        Ok(())
    }

    ///
    pub fn selection(&self) -> Option<FileTreeItem> {
        self.files.selection()
    }

    ///
    pub fn focus_select(&mut self, focus: bool) {
        self.files.focus(focus);
        self.files.show_selection(focus);
    }

    /// returns true if list is empty
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    ///
    pub fn is_file_seleted(&self) -> bool {
        self.files.is_file_seleted()
    }

    fn index_add_remove(&mut self) -> Result<bool> {
        if let Some(tree_item) = self.selection() {
            if self.is_working_dir {
                if let FileTreeItemKind::File(i) = tree_item.kind {
                    let path = Path::new(i.path.as_str());
                    match i.status {
                        StatusItemType::Deleted => {
                            sync::stage_addremoved(CWD, path)?;
                        }
                        _ => sync::stage_add_file(CWD, path)?,
                    };

                    if self.is_empty() {
                        self.queue.borrow_mut().push_back(
                            InternalEvent::StatusLastFileMoved,
                        );
                    }

                    return Ok(true);
                }

                //TODO: check if we can handle the one file case with it aswell
                sync::stage_add_all(
                    CWD,
                    tree_item.info.full_path.as_str(),
                )?;

                return Ok(true);
            }

            let path = tree_item.info.full_path.as_str();
            sync::reset_stage(CWD, path)?;
            return Ok(true);
        }

        Ok(false)
    }

    fn index_add_all(&mut self) -> Result<()> {
        sync::stage_add_all(CWD, "*")?;

        self.queue
            .borrow_mut()
            .push_back(InternalEvent::Update(NeedsUpdate::ALL));

        Ok(())
    }

    fn stage_remove_all(&mut self) -> Result<()> {
        sync::reset_stage(CWD, "*")?;

        self.queue
            .borrow_mut()
            .push_back(InternalEvent::Update(NeedsUpdate::ALL));

        Ok(())
    }

    fn dispatch_reset_workdir(&mut self) -> bool {
        if let Some(tree_item) = self.selection() {
            let is_folder =
                matches!(tree_item.kind, FileTreeItemKind::Path(_));
            self.queue.borrow_mut().push_back(
                InternalEvent::ConfirmAction(Action::Reset(
                    ResetItem {
                        path: tree_item.info.full_path,
                        is_folder,
                    },
                )),
            );

            return true;
        }
        false
    }

    fn add_to_ignore(&mut self) -> bool {
        if let Some(tree_item) = self.selection() {
            if let Err(e) =
                sync::add_to_ignore(CWD, &tree_item.info.full_path)
            {
                self.queue.borrow_mut().push_back(
                    InternalEvent::ShowErrorMsg(format!(
                        "ignore error:\n{}\nfile:\n{:?}",
                        e, tree_item.info.full_path
                    )),
                );
            } else {
                self.queue.borrow_mut().push_back(
                    InternalEvent::Update(NeedsUpdate::ALL),
                );

                return true;
            }
        }

        false
    }
}

impl DrawableComponent for ChangesComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        r: Rect,
    ) -> Result<()> {
        self.files.draw(f, r)?;

        Ok(())
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
                strings::commands::stage_all(&self.key_config),
                some_selection,
                self.focused(),
            ));
            out.push(CommandInfo::new(
                strings::commands::stage_item(&self.key_config),
                some_selection,
                self.focused(),
            ));
            out.push(CommandInfo::new(
                strings::commands::reset_item(&self.key_config),
                some_selection,
                self.focused(),
            ));
            out.push(CommandInfo::new(
                strings::commands::ignore_item(&self.key_config),
                some_selection,
                self.focused(),
            ));
        } else {
            out.push(CommandInfo::new(
                strings::commands::unstage_item(&self.key_config),
                some_selection,
                self.focused(),
            ));
            out.push(CommandInfo::new(
                strings::commands::unstage_all(&self.key_config),
                some_selection,
                self.focused(),
            ));
            out.push(
                CommandInfo::new(
                    strings::commands::commit_open(&self.key_config),
                    !self.is_empty(),
                    self.focused() || force_all,
                )
                .order(-1),
            );
        }

        CommandBlocking::PassingOn
    }

    fn event(&mut self, ev: Event) -> Result<EventState> {
        if self.files.event(ev)?.is_consumed() {
            return Ok(EventState::Consumed);
        }

        if self.focused() {
            if let Event::Key(e) = ev {
                return if e == self.key_config.open_commit
                    && !self.is_working_dir
                    && !self.is_empty()
                {
                    self.queue
                        .borrow_mut()
                        .push_back(InternalEvent::OpenCommit);
                    Ok(EventState::Consumed)
                } else if e == self.key_config.enter {
                    try_or_popup!(
                        self,
                        "staging error:",
                        self.index_add_remove()
                    );

                    self.queue.borrow_mut().push_back(
                        InternalEvent::Update(NeedsUpdate::ALL),
                    );
                    Ok(EventState::Consumed)
                } else if e == self.key_config.status_stage_all
                    && !self.is_empty()
                {
                    if self.is_working_dir {
                        try_or_popup!(
                            self,
                            "staging all error:",
                            self.index_add_all()
                        );
                    } else {
                        self.stage_remove_all()?;
                    }
                    self.queue.borrow_mut().push_back(
                        InternalEvent::StatusLastFileMoved,
                    );
                    Ok(EventState::Consumed)
                } else if e == self.key_config.status_reset_item
                    && self.is_working_dir
                {
                    Ok(self.dispatch_reset_workdir().into())
                } else if e == self.key_config.status_ignore_file
                    && self.is_working_dir
                    && !self.is_empty()
                {
                    Ok(self.add_to_ignore().into())
                } else {
                    Ok(EventState::NotConsumed)
                };
            }
        }

        Ok(EventState::NotConsumed)
    }

    fn focused(&self) -> bool {
        self.files.focused()
    }
    fn focus(&mut self, focus: bool) {
        self.files.focus(focus);
    }
}
