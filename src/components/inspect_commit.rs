use super::{
    command_pump, event_pump, visibility_blocking, CommandBlocking,
    CommandInfo, CommitDetailsComponent, Component, DiffComponent,
    DrawableComponent,
};
use crate::{
    accessors, keys::SharedKeyConfig, queue::Queue, strings,
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
    sync::{CommitId, CommitTags},
    AsyncDiff, AsyncNotification, DiffParams, DiffType,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Clear,
    Frame,
};

pub struct InspectCommitComponent {
    commit_id: Option<CommitId>,
    tags: Option<CommitTags>,
    diff: DiffComponent,
    details: CommitDetailsComponent,
    git_diff: AsyncDiff,
    visible: bool,
    key_config: SharedKeyConfig,
}

impl DrawableComponent for InspectCommitComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        if self.is_visible() {
            let percentages = if self.diff.focused() {
                (30, 70)
            } else {
                (50, 50)
            };

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(percentages.0),
                        Constraint::Percentage(percentages.1),
                    ]
                    .as_ref(),
                )
                .split(rect);

            f.render_widget(Clear, rect);

            self.details.draw(f, chunks[0])?;
            self.diff.draw(f, chunks[1])?;
        }

        Ok(())
    }
}

impl Component for InspectCommitComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.is_visible() || force_all {
            command_pump(
                out,
                force_all,
                self.components().as_slice(),
            );

            out.push(
                CommandInfo::new(
                    strings::commands::close_popup(&self.key_config),
                    true,
                    true,
                )
                .order(1),
            );

            out.push(CommandInfo::new(
                strings::commands::diff_focus_right(&self.key_config),
                self.can_focus_diff(),
                !self.diff.focused() || force_all,
            ));

            out.push(CommandInfo::new(
                strings::commands::diff_focus_left(&self.key_config),
                true,
                self.diff.focused() || force_all,
            ));
        }

        visibility_blocking(self)
    }

    fn event(&mut self, ev: Event) -> Result<bool> {
        if self.is_visible() {
            if event_pump(ev, self.components_mut().as_mut_slice())? {
                return Ok(true);
            }

            if let Event::Key(e) = ev {
                if e == self.key_config.exit_popup {
                    self.hide();
                } else if e == self.key_config.focus_right
                    && self.can_focus_diff()
                {
                    self.details.focus(false);
                    self.diff.focus(true);
                } else if e == self.key_config.focus_left
                    && self.focused()
                {
                    self.details.focus(true);
                    self.diff.focus(false);
                } else {
                }

                // stop key event propagation
                return Ok(true);
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
        self.details.show()?;
        self.details.focus(true);
        self.diff.focus(false);
        self.update()?;
        Ok(())
    }
}

impl InspectCommitComponent {
    accessors!(self, [diff, details]);

    ///
    pub fn new(
        queue: &Queue,
        sender: &Sender<AsyncNotification>,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            details: CommitDetailsComponent::new(
                queue,
                sender,
                theme.clone(),
                key_config.clone(),
            ),
            diff: DiffComponent::new(
                queue.clone(),
                theme,
                key_config.clone(),
                true,
            ),
            commit_id: None,
            tags: None,
            git_diff: AsyncDiff::new(sender.clone()),
            visible: false,
            key_config,
        }
    }

    ///
    pub fn open(
        &mut self,
        id: CommitId,
        tags: Option<CommitTags>,
    ) -> Result<()> {
        self.commit_id = Some(id);
        self.tags = tags;
        self.show()?;

        Ok(())
    }

    ///
    pub fn any_work_pending(&self) -> bool {
        self.git_diff.is_pending() || self.details.any_work_pending()
    }

    ///
    pub fn update_git(
        &mut self,
        ev: AsyncNotification,
    ) -> Result<()> {
        if self.is_visible() {
            if let AsyncNotification::CommitFiles = ev {
                self.update()?
            } else if let AsyncNotification::Diff = ev {
                self.update_diff()?
            }
        }

        Ok(())
    }

    /// called when any tree component changed selection
    pub fn update_diff(&mut self) -> Result<()> {
        if self.is_visible() {
            if let Some(id) = self.commit_id {
                if let Some(f) = self.details.files().selection_file()
                {
                    let diff_params = DiffParams {
                        path: f.path.clone(),
                        diff_type: DiffType::Commit(id),
                    };

                    if let Some((params, last)) =
                        self.git_diff.last()?
                    {
                        if params == diff_params {
                            self.diff.update(f.path, false, last)?;
                            return Ok(());
                        }
                    }

                    self.git_diff.request(diff_params)?;
                    self.diff.clear(true)?;
                    return Ok(());
                }
            }

            self.diff.clear(false)?;
        }

        Ok(())
    }

    fn update(&mut self) -> Result<()> {
        self.details.set_commit(self.commit_id, self.tags.clone())?;
        self.update_diff()?;

        Ok(())
    }

    fn can_focus_diff(&self) -> bool {
        self.details.files().selection_file().is_some()
    }
}
