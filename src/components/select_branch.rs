use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent,
};
use crate::{
    keys::SharedKeyConfig,
    queue::{Action, InternalEvent, NeedsUpdate, Queue},
    strings, ui,
};
use asyncgit::{
    sync::{
        checkout_branch, get_branches_to_display, BranchForDisplay,
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
pub struct SelectBranchComponent {
    branch_names: Vec<BranchForDisplay>,
    visible: bool,
    selection: u16,
    queue: Queue,
    theme: SharedTheme,
    key_config: SharedKeyConfig,
}

impl DrawableComponent for SelectBranchComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        _rect: Rect,
    ) -> Result<()> {
        // Render a scrolllist of branches inside a box

        if self.visible {
            const SIZE: (u16, u16) = (50, 45);
            let scroll_threshold = SIZE.1 / 3;
            let scroll =
                self.selection.saturating_sub(scroll_threshold);

            let area =
                ui::centered_rect_absolute(SIZE.0, SIZE.1, f.size());

            f.render_widget(Clear, area);
            f.render_widget(
                Block::default()
                    .title(strings::SELECT_BRANCH_POPUP_MSG)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Thick),
                area,
            );

            let chunk = Layout::default()
                .vertical_margin(1)
                .horizontal_margin(1)
                .direction(Direction::Vertical)
                .constraints(
                    [Constraint::Min(1), Constraint::Length(1)]
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

        Ok(())
    }
}

impl Component for SelectBranchComponent {
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
                strings::commands::open_branch_create_popup(
                    &self.key_config,
                ),
                true,
                true,
            ));

            out.push(CommandInfo::new(
                strings::commands::delete_branch_popup(
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
                    self.hide()
                } else if e == self.key_config.move_down {
                    self.move_selection(true)
                } else if e == self.key_config.move_up {
                    self.move_selection(false)
                } else if e == self.key_config.enter {
                    if let Err(e) = self.switch_to_selected_branch() {
                        log::error!("switch branch error: {}", e);
                        self.queue.borrow_mut().push_back(
                            InternalEvent::ShowErrorMsg(format!(
                                "switch branch error:\n{}",
                                e
                            )),
                        );
                    }
                    self.hide()
                } else if e == self.key_config.create_branch {
                    self.queue
                        .borrow_mut()
                        .push_back(InternalEvent::CreateBranch);
                    self.hide();
                } else if e == self.key_config.delete_branch {
                    self.queue.borrow_mut().push_back(
                        InternalEvent::ConfirmAction(
                            Action::DeleteBranch(
                                self.branch_names
                                    [self.selection as usize]
                                    .reference
                                    .clone(),
                            ),
                        ),
                    );
                    /*self.queue
                    .borrow_mut()
                    .push_back(InternalEvent::DeleteBranch);*/
                    //self.hide();
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

impl SelectBranchComponent {
    pub fn new(
        queue: Queue,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            branch_names: Vec::new(),
            visible: false,
            selection: 0,
            queue,
            theme,
            key_config,
        }
    }
    /// Get all the names of the branches in the repo
    pub fn get_branch_names() -> Result<Vec<BranchForDisplay>> {
        get_branches_to_display(CWD).map_err(anyhow::Error::new)
    }

    ///
    pub fn open(&mut self) -> Result<()> {
        self.update_branches()?;
        self.show()?;

        Ok(())
    }

    ////
    pub fn update_branches(&mut self) -> Result<()> {
        self.branch_names = Self::get_branch_names()?;
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
        // total width - commit hash - branch name -"*  " - "..." = remaining width
        let commit_message_length: usize =
            width_available as usize - 8 - BRANCH_NAME_LENGTH - 3 - 3;
        let mut txt = Vec::new();

        for (i, displaybranch) in self.branch_names.iter().enumerate()
        {
            let mut commit_message =
                displaybranch.top_commit_message.clone();
            if commit_message.len() > commit_message_length {
                commit_message.truncate(commit_message_length - 3);
                commit_message += "...";
            }

            let mut branch_name = displaybranch.name.clone();
            if branch_name.len() > BRANCH_NAME_LENGTH {
                branch_name.truncate(BRANCH_NAME_LENGTH - 3);
                branch_name += "...";
            }

            let is_head_str =
                if displaybranch.is_head { "*" } else { " " };

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
                            displaybranch.top_commit_reference
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
                            displaybranch.top_commit_reference
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
    fn switch_to_selected_branch(&self) -> Result<()> {
        checkout_branch(
            asyncgit::CWD,
            &self.branch_names[self.selection as usize].reference,
        )?;
        self.queue
            .borrow_mut()
            .push_back(InternalEvent::Update(NeedsUpdate::ALL));

        Ok(())
    }
}
