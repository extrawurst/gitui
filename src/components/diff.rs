use super::{
    utils::scroll_vertical::VerticalScroll, CommandBlocking,
    Direction, DrawableComponent, ScrollType,
};
use crate::{
    components::{CommandInfo, Component, EventState},
    keys::SharedKeyConfig,
    queue::{Action, InternalEvent, NeedsUpdate, Queue, ResetItem},
    strings, try_or_popup,
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
    hash,
    sync::{self, diff::DiffLinePosition},
    DiffLine, DiffLineType, FileDiff, CWD,
};
use bytesize::ByteSize;
use crossterm::event::Event;
use std::{borrow::Cow, cell::Cell, cmp, path::Path};
use tui::{
    backend::Backend,
    layout::Rect,
    symbols,
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Default)]
struct Current {
    path: String,
    is_stage: bool,
    hash: u64,
}

///
#[derive(Clone, Copy)]
enum Selection {
    Single(usize),
    Multiple(usize, usize),
}

impl Selection {
    const fn get_start(&self) -> usize {
        match self {
            Self::Single(start) | Self::Multiple(start, _) => *start,
        }
    }

    const fn get_end(&self) -> usize {
        match self {
            Self::Single(end) | Self::Multiple(_, end) => *end,
        }
    }

    fn get_top(&self) -> usize {
        match self {
            Self::Single(start) => *start,
            Self::Multiple(start, end) => cmp::min(*start, *end),
        }
    }

    fn get_bottom(&self) -> usize {
        match self {
            Self::Single(start) => *start,
            Self::Multiple(start, end) => cmp::max(*start, *end),
        }
    }

    fn modify(&mut self, direction: Direction, max: usize) {
        let start = self.get_start();
        let old_end = self.get_end();

        *self = match direction {
            Direction::Up => {
                Self::Multiple(start, old_end.saturating_sub(1))
            }

            Direction::Down => {
                Self::Multiple(start, cmp::min(old_end + 1, max))
            }
        };
    }

    fn contains(&self, index: usize) -> bool {
        match self {
            Self::Single(start) => index == *start,
            Self::Multiple(start, end) => {
                if start <= end {
                    *start <= index && index <= *end
                } else {
                    *end <= index && index <= *start
                }
            }
        }
    }
}

///
pub struct DiffComponent {
    diff: Option<FileDiff>,
    pending: bool,
    selection: Selection,
    selected_hunk: Option<usize>,
    current_size: Cell<(u16, u16)>,
    focused: bool,
    current: Current,
    scroll: VerticalScroll,
    queue: Queue,
    theme: SharedTheme,
    key_config: SharedKeyConfig,
    is_immutable: bool,
}

impl DiffComponent {
    ///
    pub fn new(
        queue: Queue,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
        is_immutable: bool,
    ) -> Self {
        Self {
            focused: false,
            queue,
            current: Current::default(),
            pending: false,
            selected_hunk: None,
            diff: None,
            current_size: Cell::new((0, 0)),
            selection: Selection::Single(0),
            scroll: VerticalScroll::new(),
            theme,
            key_config,
            is_immutable,
        }
    }
    ///
    fn can_scroll(&self) -> bool {
        self.diff
            .as_ref()
            .map(|diff| diff.lines > 1)
            .unwrap_or_default()
    }
    ///
    pub fn current(&self) -> (String, bool) {
        (self.current.path.clone(), self.current.is_stage)
    }
    ///
    pub fn clear(&mut self, pending: bool) {
        self.current = Current::default();
        self.diff = None;
        self.scroll.reset();
        self.selection = Selection::Single(0);
        self.selected_hunk = None;
        self.pending = pending;
    }
    ///
    pub fn update(
        &mut self,
        path: String,
        is_stage: bool,
        diff: FileDiff,
    ) {
        self.pending = false;

        let hash = hash(&diff);

        if self.current.hash != hash {
            let reset_selection = self.current.path != path;

            self.current = Current {
                path,
                is_stage,
                hash,
            };

            self.diff = Some(diff);

            if reset_selection {
                self.scroll.reset();
                self.selection = Selection::Single(0);
                self.update_selection(0);
            } else {
                let old_selection = match self.selection {
                    Selection::Single(line) => line,
                    Selection::Multiple(start, _) => start,
                };
                self.update_selection(old_selection);
            }
        }
    }

    fn move_selection(&mut self, move_type: ScrollType) {
        if let Some(diff) = &self.diff {
            let max = diff.lines.saturating_sub(1) as usize;

            let new_start = match move_type {
                ScrollType::Down => {
                    self.selection.get_bottom().saturating_add(1)
                }
                ScrollType::Up => {
                    self.selection.get_top().saturating_sub(1)
                }
                ScrollType::Home => 0,
                ScrollType::End => max,
                ScrollType::PageDown => {
                    self.selection.get_bottom().saturating_add(
                        self.current_size.get().1.saturating_sub(1)
                            as usize,
                    )
                }
                ScrollType::PageUp => {
                    self.selection.get_top().saturating_sub(
                        self.current_size.get().1.saturating_sub(1)
                            as usize,
                    )
                }
            };

            self.update_selection(new_start);
        }
    }

    fn update_selection(&mut self, new_start: usize) {
        if let Some(diff) = &self.diff {
            let max = diff.lines.saturating_sub(1) as usize;
            let new_start = cmp::min(max, new_start);
            self.selection = Selection::Single(new_start);
            self.selected_hunk =
                Self::find_selected_hunk(diff, new_start);
        }
    }

    fn lines_count(&self) -> usize {
        self.diff.as_ref().map_or(0, |diff| diff.lines)
    }

    fn modify_selection(&mut self, direction: Direction) {
        if self.diff.is_some() {
            self.selection.modify(direction, self.lines_count());
        }
    }

    fn copy_selection(&self) {
        if let Some(diff) = &self.diff {
            let lines_to_copy: Vec<&str> =
                diff.hunks
                    .iter()
                    .flat_map(|hunk| hunk.lines.iter())
                    .enumerate()
                    .filter_map(|(i, line)| {
                        if self.selection.contains(i) {
                            Some(line.content.trim_matches(|c| {
                                c == '\n' || c == '\r'
                            }))
                        } else {
                            None
                        }
                    })
                    .collect();

            try_or_popup!(
                self,
                "copy to clipboard error:",
                crate::clipboard::copy_string(
                    &lines_to_copy.join("\n")
                )
            );
        }
    }

    fn find_selected_hunk(
        diff: &FileDiff,
        line_selected: usize,
    ) -> Option<usize> {
        let mut line_cursor = 0_usize;
        for (i, hunk) in diff.hunks.iter().enumerate() {
            let hunk_len = hunk.lines.len();
            let hunk_min = line_cursor;
            let hunk_max = line_cursor + hunk_len;

            let hunk_selected =
                hunk_min <= line_selected && hunk_max > line_selected;

            if hunk_selected {
                return Some(i);
            }

            line_cursor += hunk_len;
        }

        None
    }

    fn get_text(&self, width: u16, height: u16) -> Vec<Spans> {
        let mut res: Vec<Spans> = Vec::new();
        if let Some(diff) = &self.diff {
            if diff.hunks.is_empty() {
                let is_positive = diff.size_delta >= 0;
                let delta_byte_size =
                    ByteSize::b(diff.size_delta.abs() as u64);
                let sign = if is_positive { "+" } else { "-" };
                res.extend(vec![Spans::from(vec![
                    Span::raw(Cow::from("size: ")),
                    Span::styled(
                        Cow::from(format!(
                            "{}",
                            ByteSize::b(diff.sizes.0)
                        )),
                        self.theme.text(false, false),
                    ),
                    Span::raw(Cow::from(" -> ")),
                    Span::styled(
                        Cow::from(format!(
                            "{}",
                            ByteSize::b(diff.sizes.1)
                        )),
                        self.theme.text(false, false),
                    ),
                    Span::raw(Cow::from(" (")),
                    Span::styled(
                        Cow::from(format!(
                            "{}{:}",
                            sign, delta_byte_size
                        )),
                        self.theme.diff_line(
                            if is_positive {
                                DiffLineType::Add
                            } else {
                                DiffLineType::Delete
                            },
                            false,
                        ),
                    ),
                    Span::raw(Cow::from(")")),
                ])]);
            } else {
                let min = self.scroll.get_top();
                let max = min + height as usize;

                let mut line_cursor = 0_usize;
                let mut lines_added = 0_usize;

                for (i, hunk) in diff.hunks.iter().enumerate() {
                    let hunk_selected = self.focused()
                        && self
                            .selected_hunk
                            .map_or(false, |s| s == i);

                    if lines_added >= height as usize {
                        break;
                    }

                    let hunk_len = hunk.lines.len();
                    let hunk_min = line_cursor;
                    let hunk_max = line_cursor + hunk_len;

                    if Self::hunk_visible(
                        hunk_min, hunk_max, min, max,
                    ) {
                        for (i, line) in hunk.lines.iter().enumerate()
                        {
                            if line_cursor >= min
                                && line_cursor <= max
                            {
                                res.push(Self::get_line_to_add(
                                    width,
                                    line,
                                    self.focused()
                                        && self
                                            .selection
                                            .contains(line_cursor),
                                    hunk_selected,
                                    i == hunk_len as usize - 1,
                                    &self.theme,
                                ));
                                lines_added += 1;
                            }

                            line_cursor += 1;
                        }
                    } else {
                        line_cursor += hunk_len;
                    }
                }
            }
        }
        res
    }

    fn get_line_to_add<'a>(
        width: u16,
        line: &'a DiffLine,
        selected: bool,
        selected_hunk: bool,
        end_of_hunk: bool,
        theme: &SharedTheme,
    ) -> Spans<'a> {
        let style = theme.diff_hunk_marker(selected_hunk);

        let left_side_of_line = if end_of_hunk {
            Span::styled(Cow::from(symbols::line::BOTTOM_LEFT), style)
        } else {
            match line.line_type {
                DiffLineType::Header => Span::styled(
                    Cow::from(symbols::line::TOP_LEFT),
                    style,
                ),
                _ => Span::styled(
                    Cow::from(symbols::line::VERTICAL),
                    style,
                ),
            }
        };

        let trimmed =
            line.content.trim_matches(|c| c == '\n' || c == '\r');

        let filled = if selected {
            // selected line
            format!("{:w$}\n", trimmed, w = width as usize)
        } else {
            // weird eof missing eol line
            format!("{}\n", trimmed)
        };
        //TODO: allow customize tabsize
        let content = Cow::from(filled.replace("\t", "  "));

        Spans::from(vec![
            left_side_of_line,
            Span::styled(
                content,
                theme.diff_line(line.line_type, selected),
            ),
        ])
    }

    const fn hunk_visible(
        hunk_min: usize,
        hunk_max: usize,
        min: usize,
        max: usize,
    ) -> bool {
        // full overlap
        if hunk_min <= min && hunk_max >= max {
            return true;
        }

        // partly overlap
        if (hunk_min >= min && hunk_min <= max)
            || (hunk_max >= min && hunk_max <= max)
        {
            return true;
        }

        false
    }

    fn unstage_hunk(&mut self) -> Result<()> {
        if let Some(diff) = &self.diff {
            if let Some(hunk) = self.selected_hunk {
                let hash = diff.hunks[hunk].header_hash;
                sync::unstage_hunk(CWD, &self.current.path, hash)?;
                self.queue_update();
            }
        }

        Ok(())
    }

    fn stage_hunk(&mut self) -> Result<()> {
        if let Some(diff) = &self.diff {
            if let Some(hunk) = self.selected_hunk {
                if diff.untracked {
                    sync::stage_add_file(
                        CWD,
                        Path::new(&self.current.path),
                    )?;
                } else {
                    let hash = diff.hunks[hunk].header_hash;
                    sync::stage_hunk(CWD, &self.current.path, hash)?;
                }

                self.queue_update();
            }
        }

        Ok(())
    }

    fn queue_update(&self) {
        self.queue
            .as_ref()
            .borrow_mut()
            .push_back(InternalEvent::Update(NeedsUpdate::ALL));
    }

    fn reset_hunk(&self) {
        if let Some(diff) = &self.diff {
            if let Some(hunk) = self.selected_hunk {
                let hash = diff.hunks[hunk].header_hash;

                self.queue.as_ref().borrow_mut().push_back(
                    InternalEvent::ConfirmAction(Action::ResetHunk(
                        self.current.path.clone(),
                        hash,
                    )),
                );
            }
        }
    }

    fn reset_lines(&self) {
        self.queue.as_ref().borrow_mut().push_back(
            InternalEvent::ConfirmAction(Action::ResetLines(
                self.current.path.clone(),
                self.selected_lines(),
            )),
        );
    }

    fn stage_lines(&self) {
        if let Some(diff) = &self.diff {
            //TODO: support untracked files aswell
            if !diff.untracked {
                let selected_lines = self.selected_lines();

                try_or_popup!(
                    self,
                    "(un)stage lines:",
                    sync::stage_lines(
                        CWD,
                        &self.current.path,
                        self.is_stage(),
                        &selected_lines,
                    )
                );

                self.queue_update();
            }
        }
    }

    fn selected_lines(&self) -> Vec<DiffLinePosition> {
        self.diff
            .as_ref()
            .map(|diff| {
                diff.hunks
                    .iter()
                    .flat_map(|hunk| hunk.lines.iter())
                    .enumerate()
                    .filter_map(|(i, line)| {
                        let is_add_or_delete = line.line_type
                            == DiffLineType::Add
                            || line.line_type == DiffLineType::Delete;
                        if self.selection.contains(i)
                            && is_add_or_delete
                        {
                            Some(line.position)
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn reset_untracked(&self) {
        self.queue.as_ref().borrow_mut().push_back(
            InternalEvent::ConfirmAction(Action::Reset(ResetItem {
                path: self.current.path.clone(),
                is_folder: false,
            })),
        );
    }

    fn stage_unstage_hunk(&mut self) -> Result<()> {
        if self.current.is_stage {
            self.unstage_hunk()?;
        } else {
            self.stage_hunk()?;
        }

        Ok(())
    }

    const fn is_stage(&self) -> bool {
        self.current.is_stage
    }
}

impl DrawableComponent for DiffComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        r: Rect,
    ) -> Result<()> {
        self.current_size.set((
            r.width.saturating_sub(2),
            r.height.saturating_sub(2),
        ));

        let current_height = self.current_size.get().1;

        self.scroll.update(
            self.selection.get_end(),
            self.lines_count(),
            usize::from(current_height),
        );

        let title = format!(
            "{}{}",
            strings::title_diff(&self.key_config),
            self.current.path
        );

        let txt = if self.pending {
            vec![Spans::from(vec![Span::styled(
                Cow::from(strings::loading_text(&self.key_config)),
                self.theme.text(false, false),
            )])]
        } else {
            self.get_text(r.width, current_height)
        };

        f.render_widget(
            Paragraph::new(txt).block(
                Block::default()
                    .title(Span::styled(
                        title.as_str(),
                        self.theme.title(self.focused),
                    ))
                    .borders(Borders::ALL)
                    .border_style(self.theme.block(self.focused)),
            ),
            r,
        );

        if self.focused {
            self.scroll.draw(f, r, &self.theme);
        }

        Ok(())
    }
}

impl Component for DiffComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        _force_all: bool,
    ) -> CommandBlocking {
        out.push(CommandInfo::new(
            strings::commands::scroll(&self.key_config),
            self.can_scroll(),
            self.focused,
        ));

        out.push(
            CommandInfo::new(
                strings::commands::diff_home_end(&self.key_config),
                self.can_scroll(),
                self.focused,
            )
            .hidden(),
        );

        if !self.is_immutable {
            out.push(CommandInfo::new(
                strings::commands::diff_hunk_remove(&self.key_config),
                self.selected_hunk.is_some(),
                self.focused && self.is_stage(),
            ));
            out.push(CommandInfo::new(
                strings::commands::diff_hunk_add(&self.key_config),
                self.selected_hunk.is_some(),
                self.focused && !self.is_stage(),
            ));
            out.push(CommandInfo::new(
                strings::commands::diff_hunk_revert(&self.key_config),
                self.selected_hunk.is_some(),
                self.focused && !self.is_stage(),
            ));
            out.push(CommandInfo::new(
                strings::commands::diff_lines_revert(
                    &self.key_config,
                ),
                //TODO: only if any modifications are selected
                true,
                self.focused && !self.is_stage(),
            ));
            out.push(CommandInfo::new(
                strings::commands::diff_lines_stage(&self.key_config),
                //TODO: only if any modifications are selected
                true,
                self.focused && !self.is_stage(),
            ));
            out.push(CommandInfo::new(
                strings::commands::diff_lines_unstage(
                    &self.key_config,
                ),
                //TODO: only if any modifications are selected
                true,
                self.focused && self.is_stage(),
            ));
        }

        out.push(CommandInfo::new(
            strings::commands::copy(&self.key_config),
            true,
            self.focused,
        ));

        CommandBlocking::PassingOn
    }

    #[allow(clippy::cognitive_complexity)]
    fn event(&mut self, ev: Event) -> Result<EventState> {
        if self.focused {
            if let Event::Key(e) = ev {
                return if e == self.key_config.move_down {
                    self.move_selection(ScrollType::Down);
                    Ok(EventState::Consumed)
                } else if e == self.key_config.shift_down {
                    self.modify_selection(Direction::Down);
                    Ok(EventState::Consumed)
                } else if e == self.key_config.shift_up {
                    self.modify_selection(Direction::Up);
                    Ok(EventState::Consumed)
                } else if e == self.key_config.end {
                    self.move_selection(ScrollType::End);
                    Ok(EventState::Consumed)
                } else if e == self.key_config.home {
                    self.move_selection(ScrollType::Home);
                    Ok(EventState::Consumed)
                } else if e == self.key_config.move_up {
                    self.move_selection(ScrollType::Up);
                    Ok(EventState::Consumed)
                } else if e == self.key_config.page_up {
                    self.move_selection(ScrollType::PageUp);
                    Ok(EventState::Consumed)
                } else if e == self.key_config.page_down {
                    self.move_selection(ScrollType::PageDown);
                    Ok(EventState::Consumed)
                } else if e == self.key_config.enter
                    && !self.is_immutable
                {
                    try_or_popup!(
                        self,
                        "hunk error:",
                        self.stage_unstage_hunk()
                    );

                    Ok(EventState::Consumed)
                } else if e == self.key_config.status_reset_item
                    && !self.is_immutable
                    && !self.is_stage()
                {
                    if let Some(diff) = &self.diff {
                        if diff.untracked {
                            self.reset_untracked();
                        } else {
                            self.reset_hunk();
                        }
                    }
                    Ok(EventState::Consumed)
                } else if e == self.key_config.diff_stage_lines
                    && !self.is_immutable
                {
                    self.stage_lines();
                    Ok(EventState::Consumed)
                } else if e == self.key_config.diff_reset_lines
                    && !self.is_immutable
                    && !self.is_stage()
                {
                    if let Some(diff) = &self.diff {
                        //TODO: reset untracked lines
                        if !diff.untracked {
                            self.reset_lines();
                        }
                    }
                    Ok(EventState::Consumed)
                } else if e == self.key_config.copy {
                    self.copy_selection();
                    Ok(EventState::Consumed)
                } else {
                    Ok(EventState::NotConsumed)
                };
            }
        }

        Ok(EventState::NotConsumed)
    }

    fn focused(&self) -> bool {
        self.focused
    }
    fn focus(&mut self, focus: bool) {
        self.focused = focus;
    }
}
