use crate::{
    components::{
        visibility_blocking, CommandBlocking, CommandInfo,
        CommitList, Component, DrawableComponent,
    },
    strings,
    ui::style::Theme,
};
use anyhow::Result;
use asyncgit::{sync, AsyncLog, AsyncNotification, FetchStatus, CWD};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use tui::{backend::Backend, layout::Rect, Frame};

const SLICE_SIZE: usize = 1200;

///
pub struct Revlog {
    list: CommitList,
    git_log: AsyncLog,
    visible: bool,
}

impl Revlog {
    ///
    pub fn new(
        sender: &Sender<AsyncNotification>,
        theme: &Theme,
    ) -> Self {
        Self {
            list: CommitList::new(strings::LOG_TITLE, theme),
            git_log: AsyncLog::new(sender.clone()),
            visible: false,
        }
    }

    ///
    pub fn any_work_pending(&self) -> bool {
        self.git_log.is_pending()
    }

    ///
    pub fn update(&mut self) -> Result<()> {
        if self.visible {
            let log_changed =
                self.git_log.fetch()? == FetchStatus::Started;

            self.list.set_count_total(self.git_log.count()?);

            let selection = self.list.selection();
            let selection_max = self.list.selection_max();
            if self.list.items().needs_data(selection, selection_max)
                || log_changed
            {
                self.fetch_commits()?;
            }

            if self.list.tags().is_empty() {
                self.list.set_tags(sync::get_tags(CWD)?);
            }
        }

        Ok(())
    }

    fn fetch_commits(&mut self) -> Result<()> {
        let want_min =
            self.list.selection().saturating_sub(SLICE_SIZE / 2);

        let commits = sync::get_commits_info(
            CWD,
            &self.git_log.get_slice(want_min, SLICE_SIZE)?,
            self.list.current_size().0.into(),
        );

        if let Ok(commits) = commits {
            self.list.items().set_items(want_min, commits);
        }

        Ok(())
    }
}

impl DrawableComponent for Revlog {
    fn draw<B: Backend>(
        &mut self,
        f: &mut Frame<B>,
        area: Rect,
    ) -> Result<()> {
        self.list.draw(f, area)?;

        Ok(())
    }
}

impl Component for Revlog {
    fn event(&mut self, ev: Event) -> Result<bool> {
        if self.visible {
            let needs_update = self.list.event(ev)?;

            if needs_update {
                self.update()?;
            }

            return Ok(needs_update);
        }

        Ok(false)
    }

    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.visible || force_all {
            self.list.commands(out, force_all);
        }

        visibility_blocking(self)
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hide(&mut self) {
        self.visible = false;
        self.git_log.set_background();
    }

    fn show(&mut self) -> Result<()> {
        self.visible = true;
        self.list.items().clear();
        self.update()?;

        Ok(())
    }
}
