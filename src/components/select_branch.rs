use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent,
};
use crate::{
    components::ScrollType,
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
use std::{
    convert::{TryFrom, TryInto},
    time::Instant,
};
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
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
    scroll_state: (Instant, f32),
    queue: Queue,
    theme: SharedTheme,
    key_config: SharedKeyConfig,
}

impl DrawableComponent for SelectBranchComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        if self.visible {
            const PERCENT_SIZE: (u16, u16) = (60, 25);
            const MIN_SIZE: (u16, u16) = (50, 20);

            let area = ui::centered_rect(
                PERCENT_SIZE.0,
                PERCENT_SIZE.1,
                f.size(),
            );
            let area = ui::rect_min(MIN_SIZE.0, MIN_SIZE.1, area);
            let area = area.intersection(rect);

            let scroll_threshold = area.height - 1;
            let scroll =
                self.selection.saturating_sub(scroll_threshold);

            f.render_widget(Clear, area);
            f.render_widget(
                Paragraph::new(
                    self.get_text(&self.theme, area.width)?,
                )
                .block(
                    Block::default()
                        .title(strings::SELECT_BRANCH_POPUP_MSG)
                        .borders(Borders::ALL)
                        .border_type(BorderType::Thick),
                )
                .scroll((scroll, 0))
                .alignment(Alignment::Left),
                area,
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
                !self.selection_is_cur_branch(),
                true,
            ));

            out.push(CommandInfo::new(
                strings::commands::rename_branch_popup(
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
                    return self.move_selection(ScrollType::Up);
                } else if e == self.key_config.move_up {
                    return self.move_selection(ScrollType::Down);
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
                } else if e == self.key_config.rename_branch {
                    let cur_branch =
                        &self.branch_names[self.selection as usize];
                    self.queue.borrow_mut().push_back(
                        InternalEvent::RenameBranch(
                            cur_branch.reference.clone(),
                            cur_branch.name.clone(),
                        ),
                    );
                    self.hide();
                } else if e == self.key_config.delete_branch
                    && !self.selection_is_cur_branch()
                {
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
            scroll_state: (Instant::now(), 0_f32),
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
    pub fn selection_is_cur_branch(&self) -> bool {
        self.branch_names
            .iter()
            .enumerate()
            .filter(|(index, b)| {
                b.is_head && *index == self.selection as usize
            })
            .count()
            > 0
    }

    ///
    fn move_selection(&mut self, scroll: ScrollType) -> Result<bool> {
        self.update_scroll_speed();

        #[allow(clippy::cast_possible_truncation)]
        let speed_int =
            u16::try_from(self.scroll_state.1 as i64)?.max(1);

        let num_branches: u16 = self.branch_names.len().try_into()?;
        let num_branches = num_branches.saturating_sub(1);

        let mut new_selection = match scroll {
            ScrollType::Up => {
                self.selection.saturating_add(speed_int)
            }
            ScrollType::Down => {
                self.selection.saturating_sub(speed_int)
            }
            _ => self.selection,
        };

        if new_selection > num_branches {
            new_selection = num_branches;
        }

        self.selection = new_selection;

        Ok(true)
    }

    ///
    fn update_scroll_speed(&mut self) {
        const REPEATED_SCROLL_THRESHOLD_MILLIS: u128 = 300;
        const SCROLL_SPEED_START: f32 = 0.1_f32;
        const SCROLL_SPEED_MAX: f32 = 10_f32;
        const SCROLL_SPEED_MULTIPLIER: f32 = 1.05_f32;

        let now = Instant::now();

        let since_last_scroll =
            now.duration_since(self.scroll_state.0);

        self.scroll_state.0 = now;

        let speed = if since_last_scroll.as_millis()
            < REPEATED_SCROLL_THRESHOLD_MILLIS
        {
            self.scroll_state.1 * SCROLL_SPEED_MULTIPLIER
        } else {
            SCROLL_SPEED_START
        };

        self.scroll_state.1 = speed.min(SCROLL_SPEED_MAX);
    }

    /// Get branches to display
    fn get_text(
        &self,
        theme: &SharedTheme,
        width_available: u16,
    ) -> Result<Text> {
        const COMMIT_HASH_LENGTH: usize = 8;
        const IS_HEAD_STAR_LENGTH: usize = 3; // "*  "
        const THREE_DOTS_LENGTH: usize = 3; // "..."

        // branch name = 30% of area size
        let branch_name_length: usize =
            width_available as usize * 30 / 100;
        // commit message takes up the remaining width
        let commit_message_length: usize = (width_available as usize)
            .saturating_sub(COMMIT_HASH_LENGTH)
            .saturating_sub(branch_name_length)
            .saturating_sub(IS_HEAD_STAR_LENGTH)
            .saturating_sub(THREE_DOTS_LENGTH);
        let mut txt = Vec::new();

        for (i, displaybranch) in self.branch_names.iter().enumerate()
        {
            let mut commit_message =
                displaybranch.top_commit_message.clone();
            if commit_message.len() > commit_message_length {
                commit_message.truncate(
                    commit_message_length
                        .saturating_sub(THREE_DOTS_LENGTH),
                );
                commit_message += "...";
            }

            let mut branch_name = displaybranch.name.clone();
            if branch_name.len() > branch_name_length {
                branch_name.truncate(
                    branch_name_length
                        .saturating_sub(THREE_DOTS_LENGTH),
                );
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
                            w = branch_name_length
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
                            w = branch_name_length
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
