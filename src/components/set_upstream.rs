use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent,
};
use crate::{
    keys::SharedKeyConfig,
    queue::{InternalEvent, NeedsUpdate, Queue},
    strings, ui,
};
//Action
use asyncgit::{
    sync::{
        get_branch_upstream, get_remote_branches_to_display,
        set_branch_upstream, BranchForDisplay, CommitId,
    },
    CWD,
};
use crossterm::event::Event;
use std::{cmp, convert::TryFrom};
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Span, Spans, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use anyhow::Result;
use ui::style::SharedTheme;

///
pub struct SetUpstreamComponent {
    branch_names: Vec<BranchForDisplay>,
    in_memory_branches: Vec<BranchForDisplay>,
    cur_local_branch_ref: Option<String>,
    visible: bool,
    selection: u16,
    queue: Queue,
    theme: SharedTheme,
    key_config: SharedKeyConfig,
}

impl DrawableComponent for SetUpstreamComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        _rect: Rect,
    ) -> Result<()> {
        // Render a scrolllist of branches inside a box

        if self.visible {
            const SIZE: (u16, u16) = (50, 20);
            let scroll_threshold = SIZE.1 / 3;
            let scroll =
                self.selection.saturating_sub(scroll_threshold);

            let area =
                ui::centered_rect_absolute(SIZE.0, SIZE.1, f.size());
            if let Some(local_branch_ref) = &self.cur_local_branch_ref
            {
                if let Some(branch_name) =
                    local_branch_ref.clone().rsplit('/').next()
                {
                    f.render_widget(Clear, area);
                    f.render_widget(
                        Block::default()
                            .title(
                                strings::set_branch_upstream_popup(
                                    branch_name,
                                ),
                            )
                            .borders(Borders::ALL)
                            .border_type(BorderType::Thick),
                        area,
                    );

                    let chunk = Layout::default()
                        .vertical_margin(1)
                        .horizontal_margin(1)
                        .direction(Direction::Vertical)
                        .constraints(
                            [
                                Constraint::Min(1),
                                Constraint::Length(1),
                            ]
                            .as_ref(),
                        )
                        .split(area)[0];
                    f.render_widget(
                        Paragraph::new(
                            self.get_text(&self.theme, area.width)?,
                        )
                        .scroll((scroll, 0))
                        .alignment(Alignment::Left),
                        chunk,
                    );
                }
            }
        }

        Ok(())
    }
}

impl Component for SetUpstreamComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.visible || force_all {
            out.clear();

            out.push(CommandInfo::new(
                strings::commands::scroll(&self.key_config),
                true,
                true,
            ));

            out.push(CommandInfo::new(
                strings::commands::close_popup(&self.key_config),
                true,
                true,
            ));

            out.push(CommandInfo::new(
                strings::commands::open_create_remote_branch_popup(
                    &self.key_config,
                ),
                true,
                true,
            ));

            out.push(CommandInfo::new(
                strings::commands::add_remote_popup(&self.key_config),
                true,
                true,
            ));

            out.push(CommandInfo::new(
                strings::commands::remove_remote_popup(
                    &self.key_config,
                ),
                true,
                true,
            ));
        }
        visibility_blocking(self)
    }

    fn event(&mut self, ev: Event) -> Result<bool> {
        if self.visible {
            if let Event::Key(e) = ev {
                if e == self.key_config.exit_popup {
                    self.hide();
                    self.queue
                        .borrow_mut()
                        .push_back(InternalEvent::SelectBranch);
                } else if e == self.key_config.move_down {
                    self.move_selection(true)
                } else if e == self.key_config.move_up {
                    self.move_selection(false)
                } else if e == self.key_config.enter {
                    if let Err(e) = self.set_upstream() {
                        log::error!(
                            "set branch upstream error: {}",
                            e
                        );
                        self.queue.borrow_mut().push_back(
                            InternalEvent::ShowErrorMsg(format!(
                                "set branch upstream error:\n{}",
                                e
                            )),
                        );
                    }
                    self.hide()
                } else if e == self.key_config.create_upstream_branch
                {
                    if let Some(local_branch_ref) =
                        &self.cur_local_branch_ref
                    {
                        self.queue.borrow_mut().push_back(
                            InternalEvent::CreateUpstreamBranch(
                                local_branch_ref.clone(),
                            ),
                        );
                        self.hide();
                    }
                } else if e == self.key_config.add_remote {
                    self.queue
                        .borrow_mut()
                        .push_back(InternalEvent::AddRemote);
                    self.hide();
                }
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hide(&mut self) {
        self.visible = false
    }

    fn show(&mut self) -> Result<()> {
        self.visible = true;

        Ok(())
    }
}

impl SetUpstreamComponent {
    pub fn new(
        queue: Queue,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            branch_names: Vec::new(),
            in_memory_branches: Vec::new(),
            cur_local_branch_ref: None,
            visible: false,
            selection: 0,
            queue,
            theme,
            key_config,
        }
    }
    /// Get all the names of the branches in the repo
    pub fn get_branch_names() -> Result<Vec<BranchForDisplay>> {
        get_remote_branches_to_display(CWD)
            .map_err(anyhow::Error::new)
    }

    ///
    pub fn open(&mut self, local_branch_ref: String) -> Result<()> {
        self.branch_names.clear();
        if let Err(e) = self.update_branches() {
            self.queue.borrow_mut().push_back(
                InternalEvent::ShowErrorMsg(format!(
                    "Opening Upstreams Error: {}",
                    e
                )),
            );
        }
        self.cur_local_branch_ref = Some(local_branch_ref);
        self.show()?;

        Ok(())
    }

    ////
    pub fn update_branches(&mut self) -> Result<()> {
        self.branch_names = Self::get_branch_names()?;
        self.branch_names
            .append(&mut (self.in_memory_branches.clone()));

        Ok(())
    }

    ////
    pub fn add_in_memory_remote_branch(
        &mut self,
        remote_branch: String,
    ) -> Result<()> {
        let branch_to_add = BranchForDisplay {
            name: remote_branch.clone(),
            top_commit: CommitId::zero(),
            top_commit_message: "NONE".to_string(),
            reference: remote_branch,
            is_head: false,
        };
        self.in_memory_branches.push(branch_to_add);
        Ok(())
    }

    ///
    fn move_selection(&mut self, inc: bool) {
        let mut new_selection = self.selection;

        new_selection = if inc {
            new_selection.saturating_add(1)
        } else {
            new_selection.saturating_sub(1)
        };
        new_selection = cmp::max(new_selection, 0);

        if let Ok(max) =
            u16::try_from(self.branch_names.len().saturating_sub(1))
        {
            self.selection = cmp::min(new_selection, max);
        }
    }

    /// Get branches to display
    fn get_text(
        &self,
        theme: &SharedTheme,
        width_available: u16,
    ) -> Result<Text> {
        const BRANCH_NAME_LENGTH: usize = 15;

        let mut upstream_ref = None;
        if let Some(local_branch_ref) = &self.cur_local_branch_ref {
            if let Ok(cur_upstream) =
                get_branch_upstream(CWD, local_branch_ref)
            {
                upstream_ref = Some(cur_upstream);
            }
        }

        // total width - commit hash - branch name -"*  " - "..." = remaining width
        let commit_message_length: usize = (width_available as usize)
            .saturating_sub(8)
            .saturating_sub(BRANCH_NAME_LENGTH)
            .saturating_sub(3)
            .saturating_sub(3);
        let mut txt = Vec::new();

        for (i, displaybranch) in self.branch_names.iter().enumerate()
        {
            let mut commit_message =
                displaybranch.top_commit_message.clone();
            if commit_message.len() > commit_message_length {
                commit_message.truncate(
                    commit_message_length.saturating_sub(3),
                );
                commit_message += "...";
            }

            let mut branch_name = displaybranch.name.clone();
            if branch_name.len() > BRANCH_NAME_LENGTH {
                branch_name.truncate(BRANCH_NAME_LENGTH - 3);
                branch_name += "...";
            }

            let is_head_str = upstream_ref.as_ref().map_or(
                " ",
                |cur_upstream_ref| {
                    if *cur_upstream_ref == displaybranch.reference {
                        "U"
                    } else {
                        " "
                    }
                },
            );

            txt.push(Spans::from(if self.selection as usize == i {
                vec![
                    Span::styled(
                        format!("{} ", is_head_str),
                        theme.commit_author(true),
                    ),
                    Span::styled(
                        format!(
                            ">{:w$} ",
                            branch_name,
                            w = BRANCH_NAME_LENGTH
                        ),
                        theme.commit_author(true),
                    ),
                    Span::styled(
                        format!(
                            "{} ",
                            displaybranch
                                .top_commit
                                .get_short_string()
                        ),
                        theme.commit_hash(true),
                    ),
                    Span::styled(
                        commit_message.to_string(),
                        theme.text(true, true),
                    ),
                ]
            } else {
                vec![
                    Span::styled(
                        format!("{} ", is_head_str),
                        theme.commit_author(false),
                    ),
                    Span::styled(
                        format!(
                            " {:w$} ",
                            branch_name,
                            w = BRANCH_NAME_LENGTH
                        ),
                        theme.commit_author(false),
                    ),
                    Span::styled(
                        format!(
                            "{} ",
                            displaybranch
                                .top_commit
                                .get_short_string()
                        ),
                        theme.commit_hash(false),
                    ),
                    Span::styled(
                        commit_message.to_string(),
                        theme.text(true, false),
                    ),
                ]
            }));
        }

        Ok(Text::from(txt))
    }

    ///
    fn set_upstream(&self) -> Result<()> {
        if let Some(local_branch_ref) = &self.cur_local_branch_ref {
            set_branch_upstream(
                asyncgit::CWD,
                local_branch_ref,
                &self.branch_names[self.selection as usize].name,
            )?;
        }
        self.queue
            .borrow_mut()
            .push_back(InternalEvent::Update(NeedsUpdate::ALL));

        Ok(())
    }
}
