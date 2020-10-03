use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent,
};
use crate::{
    keys::SharedKeyConfig,
    queue::{InternalEvent, NeedsUpdate, Queue},
    strings, ui,
    version::Version,
};
use asyncgit::hash;
use crossterm::event::Event;
use itertools::Itertools;
use std::{borrow::Cow, cmp, convert::TryFrom};
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Text},
    Frame,
};

use asyncgit::sync::{
    checkout_branch, get_branches_to_display, BranchForDisplay,
};

use anyhow::Result;
use ui::style::SharedTheme;

///
pub struct SelectBranchComponent {
    branch_names: Vec<BranchForDisplay>,
    //cur_branch: String,
    visible: bool,
    selection: u16,
    can_create_branch: bool,
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
        // Render a scrolllist inside a box

        if self.visible {
            const SIZE: (u16, u16) = (50, 20); //(65, 24);
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
                Paragraph::new(self.get_text(&self.theme).iter())
                    .scroll(scroll)
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
                    self.switch_to_selected_branch();
                    self.hide()
                } else if e == self.key_config.create_branch {
                    self.queue
                        .borrow_mut()
                        .push_back(InternalEvent::CreateBranch);
                } else {
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
            branch_names: Self::get_branch_names(), //update_branches,
            //cur_branch: get_branch_name(),
            visible: false,
            selection: 0,
            can_create_branch: true,
            queue,
            theme,
            key_config,
        }
    }
    /// Get all the names of the branches in the repo
    pub fn get_branch_names() -> Vec<BranchForDisplay> {
        get_branches_to_display("./")
    }

    pub fn open(&mut self, selected_branch: String) -> Result<()> {
        self.show()?;

        Ok(())
    }

    pub fn update_branches(&mut self) {
        self.branch_names = Self::get_branch_names();
        //self.cur_branch = get_branch_name("./");
    }

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

    fn get_text(&self, theme: &SharedTheme) -> Vec<Text> {
        let mut txt = Vec::new();

        let max_branch_name = self
            .branch_names
            .iter()
            .map(|displaybranch| displaybranch.name.len())
            .max()
            .expect("Failed to find max branch length");

        for (i, displaybranch) in self.branch_names.iter().enumerate()
        {
            let mut commit_message =
                displaybranch.top_commit_message.clone();
            commit_message.truncate(30);
            commit_message += "...";

            let is_head_str =
                if displaybranch.is_head { "*" } else { " " };

            txt.push(Text::Styled(
                if self.selection as usize == i {
                    Cow::from(format!(
                        "{} >{:w$} {} {}\n",
                        is_head_str,
                        displaybranch.name,
                        displaybranch.top_commit_reference,
                        displaybranch.top_commit_message,
                        w = max_branch_name
                    ))
                } else {
                    Cow::from(format!(
                        "{}  {:w$} {} {}\n",
                        is_head_str,
                        displaybranch.name,
                        displaybranch.top_commit_reference,
                        displaybranch.top_commit_message,
                        w = max_branch_name
                    ))
                },
                theme.text(true, i == self.selection.into()),
            ));
        }

        txt
    }

    ///
    fn switch_to_selected_branch(&self) {
        checkout_branch(
            "./",
            &self.branch_names[self.selection as usize].reference,
        );
        self.queue
            .borrow_mut()
            .push_back(InternalEvent::Update(NeedsUpdate::ALL));
    }
}
