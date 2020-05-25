use crate::{
    components::{
        visibility_blocking, CommandBlocking, CommandInfo,
        CommitList, Component, DrawableComponent,
    },
    keys,
    queue::{Action, InternalEvent, Queue},
    strings,
    ui::style::Theme,
};
use anyhow::Result;
use asyncgit::sync;
use asyncgit::CWD;
use crossterm::event::Event;
use strings::commands;
use sync::CommitId;

pub struct StashList {
    list: CommitList,
    visible: bool,
    queue: Queue,
}

impl StashList {
    ///
    pub fn new(queue: &Queue, theme: &Theme) -> Self {
        Self {
            visible: false,
            list: CommitList::new(strings::STASHLIST_TITLE, theme),
            queue: queue.clone(),
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
            match sync::stash_apply(CWD, e.id) {
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

    ///
    pub fn drop(id: CommitId) -> bool {
        sync::stash_drop(CWD, id).is_ok()
    }
}

impl DrawableComponent for StashList {
    fn draw<B: tui::backend::Backend>(
        &mut self,
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
                commands::STASHLIST_APPLY,
                selection_valid,
                true,
            ));
            out.push(CommandInfo::new(
                commands::STASHLIST_DROP,
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
                match k {
                    keys::STASH_APPLY => self.apply_stash(),
                    keys::STASH_DROP => self.drop_stash(),

                    _ => (),
                };
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
