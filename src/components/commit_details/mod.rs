mod details;

use super::{
    dialog_paragraph, CommandBlocking, CommandInfo, Component,
    DrawableComponent, FileTreeComponent,
};
use crate::{strings, ui::style::Theme};
use anyhow::Result;
use asyncgit::{
    sync::{CommitId, Tags},
    AsyncCommitFiles, AsyncNotification,
};
use crossbeam_channel::Sender;
use details::DetailsComponent;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

pub struct CommitDetailsComponent {
    details: DetailsComponent,
    tags: Vec<String>,
    file_tree: FileTreeComponent,
    theme: Theme,
    git_commit_files: AsyncCommitFiles,
    visible: bool,
}

impl CommitDetailsComponent {
    ///
    pub fn new(
        sender: &Sender<AsyncNotification>,
        theme: &Theme,
    ) -> Self {
        Self {
            details: DetailsComponent::new(theme),
            theme: *theme,
            tags: Vec::new(),
            git_commit_files: AsyncCommitFiles::new(sender),
            file_tree: FileTreeComponent::new("", false, None, theme),
            visible: false,
        }
    }

    fn get_files_title(&self) -> String {
        let files_loading = self.git_commit_files.is_pending();
        let files_count = self.file_tree.file_count();

        if files_loading {
            strings::commit::DETAILS_FILES_LOADING_TITLE.to_string()
        } else {
            format!(
                "{} {}",
                strings::commit::DETAILS_FILES_TITLE,
                files_count
            )
        }
    }

    ///
    pub fn set_commit(
        &mut self,
        id: Option<CommitId>,
        tags: &Tags,
    ) -> Result<()> {
        self.details.set_commit(id, tags);

        if let Some(id) = id {
            if let Some((fetched_id, res)) =
                self.git_commit_files.current()?
            {
                if fetched_id == id {
                    self.file_tree.update(res.as_slice())?;
                } else {
                    self.file_tree.clear()?;
                    self.git_commit_files.fetch(id)?;
                }
            } else {
                self.file_tree.clear()?;
                self.git_commit_files.fetch(id)?;
            }
        }

        Ok(())
    }

    ///
    pub fn any_work_pending(&self) -> bool {
        self.git_commit_files.is_pending()
    }
}

impl DrawableComponent for CommitDetailsComponent {
    fn draw<B: Backend>(
        &mut self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage(70),
                    Constraint::Percentage(30),
                ]
                .as_ref(),
            )
            .split(rect);

        self.details.draw(f, chunks[0])?;

        self.file_tree.set_title(self.get_files_title());
        self.file_tree.draw(f, chunks[1])?;

        Ok(())
    }
}

impl Component for CommitDetailsComponent {
    fn commands(
        &self,
        _out: &mut Vec<CommandInfo>,
        _force_all: bool,
    ) -> CommandBlocking {
        unimplemented!()
    }

    fn event(
        &mut self,
        _ev: crossterm::event::Event,
    ) -> Result<bool> {
        unimplemented!()
    }

    ///
    fn is_visible(&self) -> bool {
        self.visible
    }
    ///
    fn hide(&mut self) {
        self.visible = false;
    }
    ///
    fn show(&mut self) -> Result<()> {
        self.visible = true;
        Ok(())
    }
}
