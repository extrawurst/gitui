use super::{
    CommandBlocking, Direction, DrawableComponent, ScrollType,
};
use crate::{
    components::{CommandInfo, Component},
    keys::SharedKeyConfig,
    queue::{Action, InternalEvent, NeedsUpdate, Queue, ResetItem},
    strings, try_or_popup,
    ui::{self, calc_scroll_top, style::SharedTheme},
};
use anyhow::Result;
use asyncgit::{hash, sync, DiffLine, DiffLineType, FileDiff, CWD};
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
    scroll_top: Cell<usize>,
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
            scroll_top: Cell::new(0),
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
    pub fn clear(&mut self, pending: bool) -> Result<()> {
        self.current = Current::default();
        self.diff = None;
        self.scroll_top.set(0);
        self.selection = Selection::Single(0);
        self.selected_hunk = None;
        self.pending = pending;

        Ok(())
    }
    ///
    pub fn update(
        &mut self,
        path: String,
        is_stage: bool,
        diff: FileDiff,
    ) -> Result<()> {
        self.pending = false;

        let hash = hash(&diff);

        if self.current.hash != hash {
            self.current = Current {
                path,
                is_stage,
                hash,
            };

            self.selected_hunk = Self::find_selected_hunk(
                &diff,
                self.selection.get_start(),
            )?;

            self.diff = Some(diff);
            self.scroll_top.set(0);
            self.selection = Selection::Single(0);
        }

        Ok(())
    }

    fn move_selection(
        &mut self,
        move_type: ScrollType,
    ) -> Result<()> {
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

            let new_start = cmp::min(max, new_start);

            self.selection = Selection::Single(new_start);

            self.selected_hunk =
                Self::find_selected_hunk(diff, new_start)?;
        }
        Ok(())
    }

    fn lines_count(&self) -> usize {
        self.diff
            .as_ref()
            .map_or(0, |diff| diff.lines.saturating_sub(1))
    }

    fn modify_selection(
        &mut self,
        direction: Direction,
    ) -> Result<()> {
        if let Some(diff) = &self.diff {
            let max = diff.lines.saturating_sub(1);

            self.selection.modify(direction, max);
        }

        Ok(())
    }

    fn copy_selection(&self) -> Result<()> {
        if let Some(diff) = &self.diff {
            let lines_to_copy: Vec<&str> = diff
                .hunks
                .iter()
                .flat_map(|hunk| hunk.lines.iter())
                .enumerate()
                .filter_map(|(i, line)| {
                    if self.selection.contains(i) {
                        Some(
                            line.content
                                .trim_matches(|c| {
                                    c == '\n' || c == '\r'
                                })
                                .as_ref(),
                        )
                    } else {
                        None
                    }
                })
                .collect();

            try_or_popup!(
                self,
                "copy to clipboard error:",
                crate::clipboard::copy_string(
                    lines_to_copy.join("\n")
                )
            );
        }

        Ok(())
    }

    fn find_selected_hunk(
        diff: &FileDiff,
        line_selected: usize,
    ) -> Result<Option<usize>> {
        let mut line_cursor = 0_usize;
        for (i, hunk) in diff.hunks.iter().enumerate() {
            let hunk_len = hunk.lines.len();
            let hunk_min = line_cursor;
            let hunk_max = line_cursor + hunk_len;

            let hunk_selected =
                hunk_min <= line_selected && hunk_max > line_selected;

            if hunk_selected {
                return Ok(Some(i));
            }

            line_cursor += hunk_len;
        }

        Ok(None)
    }

    fn get_text(
        &self,
        width: u16,
        height: u16,
    ) -> Result<Vec<Spans>> {
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
                let min = self.scroll_top.get();
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
        Ok(res)
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
                sync::unstage_hunk(
                    CWD,
                    self.current.path.clone(),
                    hash,
                )?;
                self.queue_update();
            }
        }

        Ok(())
    }

    fn stage_hunk(&mut self) -> Result<()> {
        if let Some(diff) = &self.diff {
            if let Some(hunk) = self.selected_hunk {
                let path = self.current.path.clone();
                if diff.untracked {
                    sync::stage_add_file(CWD, Path::new(&path))?;
                } else {
                    let hash = diff.hunks[hunk].header_hash;
                    sync::stage_hunk(CWD, path, hash)?;
                }

                self.queue_update();
            }
        }

        Ok(())
    }

    fn queue_update(&mut self) {
        self.queue
            .as_ref()
            .borrow_mut()
            .push_back(InternalEvent::Update(NeedsUpdate::ALL));
    }

    fn reset_hunk(&self) -> Result<()> {
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
        Ok(())
    }

    fn reset_untracked(&self) -> Result<()> {
        self.queue.as_ref().borrow_mut().push_back(
            InternalEvent::ConfirmAction(Action::Reset(ResetItem {
                path: self.current.path.clone(),
                is_folder: false,
            })),
        );

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

        self.scroll_top.set(calc_scroll_top(
            self.scroll_top.get(),
            self.current_size.get().1 as usize,
            self.selection.get_end(),
        ));

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
            self.get_text(r.width, self.current_size.get().1)?
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
            ui::draw_scrollbar(
                f,
                r,
                &self.theme,
                self.lines_count(),
                self.selection.get_end(),
            );
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

        if crate::clipboard::is_supported() {
            out.push(CommandInfo::new(
                strings::commands::copy(&self.key_config),
                true,
                self.focused,
            ));
        }

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
        }

        CommandBlocking::PassingOn
    }

    fn event(&mut self, ev: Event) -> Result<bool> {
        if self.focused {
            if let Event::Key(e) = ev {
                return if e == self.key_config.move_down {
                    self.move_selection(ScrollType::Down)?;
                    Ok(true)
                } else if e == self.key_config.shift_down {
                    self.modify_selection(Direction::Down)?;
                    Ok(true)
                } else if e == self.key_config.shift_up {
                    self.modify_selection(Direction::Up)?;
                    Ok(true)
                } else if e == self.key_config.end {
                    self.move_selection(ScrollType::End)?;
                    Ok(true)
                } else if e == self.key_config.home {
                    self.move_selection(ScrollType::Home)?;
                    Ok(true)
                } else if e == self.key_config.move_up {
                    self.move_selection(ScrollType::Up)?;
                    Ok(true)
                } else if e == self.key_config.page_up {
                    self.move_selection(ScrollType::PageUp)?;
                    Ok(true)
                } else if e == self.key_config.page_down {
                    self.move_selection(ScrollType::PageDown)?;
                    Ok(true)
                } else if e == self.key_config.enter
                    && !self.is_immutable
                {
                    if self.current.is_stage {
                        self.unstage_hunk()?;
                    } else {
                        self.stage_hunk()?;
                    }
                    Ok(true)
                } else if e == self.key_config.status_reset_item
                    && !self.is_immutable
                    && !self.is_stage()
                {
                    if let Some(diff) = &self.diff {
                        if diff.untracked {
                            self.reset_untracked()?;
                        } else {
                            self.reset_hunk()?;
                        }
                    }
                    Ok(true)
                } else if e == self.key_config.copy
                    && crate::clipboard::is_supported()
                {
                    self.copy_selection()?;
                    Ok(true)
                } else {
                    Ok(false)
                };
            }
        }

        Ok(false)
    }

    fn focused(&self) -> bool {
        self.focused
    }
    fn focus(&mut self, focus: bool) {
        self.focused = focus
    }
}
