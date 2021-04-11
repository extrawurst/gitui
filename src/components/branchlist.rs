use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent,
};
use crate::{
    components::ScrollType,
    keys::SharedKeyConfig,
    queue::{Action, InternalEvent, NeedsUpdate, Queue},
    strings, try_or_popup,
    ui::{self, calc_scroll_top, Size},
};
use anyhow::Result;
use asyncgit::{
    sync::{
        branch::checkout_remote_branch, checkout_branch,
        get_branches_info, BranchInfo,
    },
    CWD,
};
use crossterm::event::Event;
use std::{cell::Cell, convert::TryInto};
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    text::{Span, Spans, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use ui::style::SharedTheme;
use unicode_truncate::UnicodeTruncateStr;

///
pub struct BranchListComponent {
    branches: Vec<BranchInfo>,
    local: bool,
    visible: bool,
    selection: u16,
    scroll_top: Cell<usize>,
    current_height: Cell<u16>,
    queue: Queue,
    theme: SharedTheme,
    key_config: SharedKeyConfig,
}

impl DrawableComponent for BranchListComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        if self.visible {
            const PERCENT_SIZE: Size = Size::new(80, 25);
            const MIN_SIZE: Size = Size::new(60, 20);

            let area = ui::centered_rect(
                PERCENT_SIZE.width,
                PERCENT_SIZE.height,
                f.size(),
            );
            let area =
                ui::rect_inside(MIN_SIZE, f.size().into(), area);
            let area = area.intersection(rect);

            let height_in_lines =
                (area.height as usize).saturating_sub(2);

            self.scroll_top.set(calc_scroll_top(
                self.scroll_top.get(),
                height_in_lines,
                self.selection as usize,
            ));

            f.render_widget(Clear, area);
            f.render_widget(
                Paragraph::new(self.get_text(
                    &self.theme,
                    area.width,
                    height_in_lines,
                ))
                .block(
                    Block::default()
                        .title(strings::title_branches(self.local))
                        .border_type(BorderType::Thick)
                        .borders(Borders::ALL),
                )
                .alignment(Alignment::Left),
                area,
            );

            ui::draw_scrollbar(
                f,
                area,
                &self.theme,
                self.branches.len(),
                self.scroll_top.get(),
            );

            self.current_height.set(height_in_lines.try_into()?);
        }

        Ok(())
    }
}

impl Component for BranchListComponent {
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
                strings::commands::select_branch_popup(
                    &self.key_config,
                ),
                !self.selection_is_cur_branch(),
                true,
            ));

            out.push(CommandInfo::new(
                strings::commands::open_branch_create_popup(
                    &self.key_config,
                ),
                true,
                self.local,
            ));

            out.push(CommandInfo::new(
                strings::commands::delete_branch_popup(
                    &self.key_config,
                ),
                !self.selection_is_cur_branch(),
                self.local,
            ));

            out.push(CommandInfo::new(
                strings::commands::rename_branch_popup(
                    &self.key_config,
                ),
                true,
                self.local,
            ));

            out.push(CommandInfo::new(
                strings::commands::toggle_branch_popup(
                    &self.key_config,
                    self.local,
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
                    return self.move_selection(ScrollType::Up);
                } else if e == self.key_config.move_up {
                    return self.move_selection(ScrollType::Down);
                } else if e == self.key_config.page_down {
                    return self.move_selection(ScrollType::PageDown);
                } else if e == self.key_config.page_up {
                    return self.move_selection(ScrollType::PageUp);
                } else if e == self.key_config.enter {
                    try_or_popup!(
                        self,
                        "switch branch error:",
                        self.switch_to_selected_branch()
                    );
                } else if e == self.key_config.create_branch {
                    self.queue
                        .borrow_mut()
                        .push_back(InternalEvent::CreateBranch);
                    self.hide();
                } else if e == self.key_config.rename_branch {
                    let cur_branch =
                        &self.branches[self.selection as usize];
                    self.queue.borrow_mut().push_back(
                        InternalEvent::RenameBranch(
                            cur_branch.reference.clone(),
                            cur_branch.name.clone(),
                        ),
                    );

                    self.update_branches()?;
                } else if e == self.key_config.delete_branch
                    && !self.selection_is_cur_branch()
                {
                    self.queue.borrow_mut().push_back(
                        InternalEvent::ConfirmAction(
                            Action::DeleteBranch(
                                self.branches
                                    [self.selection as usize]
                                    .reference
                                    .clone(),
                            ),
                        ),
                    );
                } else if e == self.key_config.toggle_remote_branches
                {
                    self.local = !self.local;
                    self.update_branches()?;
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

impl BranchListComponent {
    pub fn new(
        queue: Queue,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            branches: Vec::new(),
            local: true,
            visible: false,
            selection: 0,
            scroll_top: Cell::new(0),
            queue,
            theme,
            key_config,
            current_height: Cell::new(0),
        }
    }

    ///
    pub fn open(&mut self) -> Result<()> {
        self.update_branches()?;
        self.show()?;

        Ok(())
    }

    /// fetch list of branches
    pub fn update_branches(&mut self) -> Result<()> {
        self.branches = get_branches_info(CWD, self.local)?;
        //remove remote branch called `HEAD`
        if !self.local {
            self.branches
                .iter()
                .position(|b| b.name.ends_with("/HEAD"))
                .map(|idx| self.branches.remove(idx));
        }
        self.set_selection(self.selection)?;
        Ok(())
    }

    fn selection_is_cur_branch(&self) -> bool {
        self.branches
            .iter()
            .enumerate()
            .filter(|(index, b)| {
                b.local_details()
                    .map(|details| {
                        details.is_head
                            && *index == self.selection as usize
                    })
                    .unwrap_or_default()
            })
            .count()
            > 0
    }

    ///
    fn move_selection(&mut self, scroll: ScrollType) -> Result<bool> {
        let new_selection = match scroll {
            ScrollType::Up => self.selection.saturating_add(1),
            ScrollType::Down => self.selection.saturating_sub(1),
            ScrollType::PageDown => self
                .selection
                .saturating_add(self.current_height.get()),
            ScrollType::PageUp => self
                .selection
                .saturating_sub(self.current_height.get()),
            _ => self.selection,
        };

        self.set_selection(new_selection)?;

        Ok(true)
    }

    fn set_selection(&mut self, selection: u16) -> Result<()> {
        let num_branches: u16 = self.branches.len().try_into()?;
        let num_branches = num_branches.saturating_sub(1);

        let selection = if selection > num_branches {
            num_branches
        } else {
            selection
        };

        self.selection = selection;

        Ok(())
    }

    /// Get branches to display
    fn get_text(
        &self,
        theme: &SharedTheme,
        width_available: u16,
        height: usize,
    ) -> Text {
        const COMMIT_HASH_LENGTH: usize = 8;
        const IS_HEAD_STAR_LENGTH: usize = 3; // "*  "
        const THREE_DOTS_LENGTH: usize = 3; // "..."

        let branch_name_length: usize =
            width_available as usize * 40 / 100;
        // commit message takes up the remaining width
        let commit_message_length: usize = (width_available as usize)
            .saturating_sub(COMMIT_HASH_LENGTH)
            .saturating_sub(branch_name_length)
            .saturating_sub(IS_HEAD_STAR_LENGTH)
            .saturating_sub(THREE_DOTS_LENGTH);
        let mut txt = Vec::new();

        for (i, displaybranch) in self
            .branches
            .iter()
            .skip(self.scroll_top.get())
            .take(height)
            .enumerate()
        {
            let mut commit_message =
                displaybranch.top_commit_message.clone();
            if commit_message.len() > commit_message_length {
                commit_message.unicode_truncate(
                    commit_message_length
                        .saturating_sub(THREE_DOTS_LENGTH),
                );
                commit_message += "...";
            }

            let mut branch_name = displaybranch.name.clone();
            if branch_name.len()
                > branch_name_length.saturating_sub(THREE_DOTS_LENGTH)
            {
                branch_name = branch_name
                    .unicode_truncate(
                        branch_name_length
                            .saturating_sub(THREE_DOTS_LENGTH),
                    )
                    .0
                    .to_string();
                branch_name += "...";
            }

            let selected =
                self.selection as usize - self.scroll_top.get() == i;

            let is_head = displaybranch
                .local_details()
                .map(|details| details.is_head)
                .unwrap_or_default();
            let is_head_str = if is_head { "*" } else { " " };
            let has_upstream_str = if displaybranch
                .local_details()
                .map(|details| details.has_upstream)
                .unwrap_or_default()
            {
                "\u{2191}"
            } else {
                " "
            };

            let span_prefix = Span::styled(
                format!("{}{} ", is_head_str, has_upstream_str),
                theme.commit_author(selected),
            );
            let span_hash = Span::styled(
                format!(
                    "{} ",
                    displaybranch.top_commit.get_short_string()
                ),
                theme.commit_hash(selected),
            );
            let span_msg = Span::styled(
                commit_message.to_string(),
                theme.text(true, selected),
            );
            let span_name = Span::styled(
                format!(
                    "{:w$} ",
                    branch_name,
                    w = branch_name_length
                ),
                theme.branch(selected, is_head),
            );

            txt.push(Spans::from(vec![
                span_prefix,
                span_name,
                span_hash,
                span_msg,
            ]));
        }

        Text::from(txt)
    }

    ///
    fn switch_to_selected_branch(&mut self) -> Result<()> {
        if self.local {
            checkout_branch(
                asyncgit::CWD,
                &self.branches[self.selection as usize].reference,
            )?;
            self.hide()
        } else {
            checkout_remote_branch(
                CWD,
                &self.branches[self.selection as usize],
            )?;
            self.local = true;
            self.update_branches()?;
        }

        self.queue
            .borrow_mut()
            .push_back(InternalEvent::Update(NeedsUpdate::ALL));

        Ok(())
    }
}
