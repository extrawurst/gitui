use crate::{
    components::{
        visibility_blocking, CommandBlocking, CommandInfo,
        CommitList, Component, DrawableComponent,
    },
    keys::SharedKeyConfig,
    queue::{Action, InternalEvent, Queue},
    strings,
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
    sync::{self, CommitId},
    CWD,
};
use crossterm::event::Event;

pub struct StashList {
    list: CommitList,
    visible: bool,
    queue: Queue,
    key_config: SharedKeyConfig,
}

impl StashList {
    ///
    pub fn new(
        queue: &Queue,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            visible: false,
            list: CommitList::new(
                &strings::stashlist_title(&key_config),
                theme,
                key_config.clone(),
            ),
            queue: queue.clone(),
            key_config,
        }
    }

    ///
    pub fn update(&mut self) -> Result<()> {
        if self.visible {
            let stashes = sync::get_stashes(CWD)?;
            let commits =
                sync::get_commits_info(CWD, stashes.as_slice(), 100)?;

            self.list.set_count_total(commits.len());
            self.list.items().set_items(0, commits);
        }

        Ok(())
    }

    fn apply_stash(&mut self) {
        if let Some(e) = self.list.selected_entry() {
            match sync::stash_apply(CWD, e.id, false) {
                Ok(_) => {
                    self.queue
                        .borrow_mut()
                        .push_back(InternalEvent::TabSwitch);
                }
                Err(e) => {
                    self.queue.borrow_mut().push_back(
                        InternalEvent::ShowErrorMsg(format!(
                            "stash apply error:\n{}",
                            e,
                        )),
                    );
                }
            }
        }
    }

    fn drop_stash(&mut self) {
        if let Some(e) = self.list.selected_entry() {
            self.queue.borrow_mut().push_back(
                InternalEvent::ConfirmAction(Action::StashDrop(e.id)),
            );
        }
    }

    fn inspect(&mut self) {
        if let Some(e) = self.list.selected_entry() {
            self.queue
                .borrow_mut()
                .push_back(InternalEvent::InspectCommit(e.id, None));
        }
    }

    ///
    pub fn drop(id: CommitId) -> bool {
        sync::stash_drop(CWD, id).is_ok()
    }
}

impl DrawableComponent for StashList {
    fn draw<B: tui::backend::Backend>(
        &self,
        f: &mut tui::Frame<B>,
        rect: tui::layout::Rect,
    ) -> Result<()> {
        self.list.draw(f, rect)?;

        Ok(())
    }
}

impl Component for StashList {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.visible || force_all {
            self.list.commands(out, force_all);

            let selection_valid =
                self.list.selected_entry().is_some();
            out.push(CommandInfo::new(
                strings::commands::stashlist_apply(&self.key_config),
                selection_valid,
                true,
            ));
            out.push(CommandInfo::new(
                strings::commands::stashlist_drop(&self.key_config),
                selection_valid,
                true,
            ));
            out.push(CommandInfo::new(
                strings::commands::stashlist_inspect(
                    &self.key_config,
                ),
                selection_valid,
                true,
            ));
        }

        visibility_blocking(self)
    }

    fn event(&mut self, ev: crossterm::event::Event) -> Result<bool> {
        if self.visible {
            if self.list.event(ev)? {
                return Ok(true);
            }

            if let Event::Key(k) = ev {
                if k == self.key_config.enter {
                    self.apply_stash()
                } else if k == self.key_config.stash_drop {
                    self.drop_stash()
                } else if k == self.key_config.stash_open {
                    self.inspect()
                } else {
                }
            }
        }

        Ok(false)
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn show(&mut self) -> Result<()> {
        self.visible = true;
        self.update()?;
        Ok(())
    }
}
