use super::{CommandBlocking, DrawableComponent, ScrollType};
use crate::ui::style::Theme;
use crate::{
    components::{CommandInfo, Component},
    keys,
    queue::{InternalEvent, Queue},
    strings,
};
use asyncgit::{hash, DiffLine, DiffLineType, FileDiff};
use crossterm::event::Event;
use std::{borrow::Cow, cmp, convert::TryFrom};
use strings::commands;

use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    symbols,
    widgets::{Block, Borders, Paragraph, Text},
    Frame,
};

#[derive(Default)]
struct Current {
    path: String,
    is_stage: bool,
    hash: u64,
}

///
pub struct DiffComponent {
    diff: FileDiff,
    scroll: u16,
    current_height: u16,
    focused: bool,
    current: Current,
    selected_hunk: Option<u16>,
    queue: Queue,
    theme: Theme,
}

impl DiffComponent {
    ///
    pub fn new(queue: Queue, theme: Theme) -> Self {
        Self {
            focused: false,
            queue,
            current: Current::default(),
            selected_hunk: None,
            diff: FileDiff::default(),
            scroll: 0,
            current_height: 0,
            theme,
        }
    }
    ///
    fn can_scroll(&self) -> bool {
        self.diff.lines > 1
    }
    ///
    pub fn current(&self) -> (String, bool) {
        (self.current.path.clone(), self.current.is_stage)
    }
    ///
    pub fn clear(&mut self) {
        self.current = Current::default();
        self.diff = FileDiff::default();
        self.scroll = 0;

        self.selected_hunk =
            Self::find_selected_hunk(&self.diff, self.scroll);
    }
    ///
    pub fn update(
        &mut self,
        path: String,
        is_stage: bool,
        diff: FileDiff,
    ) {
        let hash = hash(&diff);

        if self.current.hash != hash {
            self.current = Current {
                path,
                is_stage,
                hash,
            };
            self.diff = diff;
            self.scroll = 0;

            self.selected_hunk =
                Self::find_selected_hunk(&self.diff, self.scroll);
        }
    }

    fn scroll(&mut self, scroll: ScrollType) {
        let old = self.scroll;

        let scroll_max = self.diff.lines.saturating_sub(1);

        self.scroll = match scroll {
            ScrollType::Down => self.scroll.saturating_add(1),
            ScrollType::Up => self.scroll.saturating_sub(1),
            ScrollType::Home => 0,
            ScrollType::End => scroll_max,
            ScrollType::PageDown => self.scroll.saturating_add(
                self.current_height.saturating_sub(1),
            ),
            ScrollType::PageUp => self.scroll.saturating_sub(
                self.current_height.saturating_sub(1),
            ),
        };

        self.scroll = cmp::min(scroll_max, self.scroll);

        if old != self.scroll {
            self.selected_hunk =
                Self::find_selected_hunk(&self.diff, self.scroll);
        }
    }

    fn find_selected_hunk(
        diff: &FileDiff,
        line_selected: u16,
    ) -> Option<u16> {
        let mut line_cursor = 0_u16;
        for (i, hunk) in diff.hunks.iter().enumerate() {
            let hunk_len = u16::try_from(hunk.lines.len()).unwrap();
            let hunk_min = line_cursor;
            let hunk_max = line_cursor + hunk_len;

            let hunk_selected =
                hunk_min <= line_selected && hunk_max > line_selected;

            if hunk_selected {
                return Some(u16::try_from(i).unwrap());
            }

            line_cursor += hunk_len;
        }

        None
    }

    fn get_text(&self, width: u16, height: u16) -> Vec<Text> {
        let selection = self.scroll;
        let height_d2 = height / 2;
        let min = self.scroll.saturating_sub(height_d2);
        let max = min + height;

        let mut res = Vec::new();
        let mut line_cursor = 0_u16;
        let mut lines_added = 0_u16;

        for (i, hunk) in self.diff.hunks.iter().enumerate() {
            let hunk_selected = self
                .selected_hunk
                .map_or(false, |s| s == u16::try_from(i).unwrap());

            if lines_added >= height {
                break;
            }

            let hunk_len = u16::try_from(hunk.lines.len()).unwrap();
            let hunk_min = line_cursor;
            let hunk_max = line_cursor + hunk_len;

            if Self::hunk_visible(hunk_min, hunk_max, min, max) {
                for (i, line) in hunk.lines.iter().enumerate() {
                    if line_cursor >= min {
                        Self::add_line(
                            &mut res,
                            width,
                            line,
                            selection == line_cursor,
                            hunk_selected,
                            i == hunk_len as usize - 1,
                            self.theme,
                        );
                        lines_added += 1;
                    }

                    line_cursor += 1;
                }
            } else {
                line_cursor += hunk_len;
            }
        }
        res
    }

    fn add_line(
        text: &mut Vec<Text>,
        width: u16,
        line: &DiffLine,
        selected: bool,
        selected_hunk: bool,
        end_of_hunk: bool,
        theme: Theme,
    ) {
        {
            let style = theme.text(false, selected || selected_hunk);

            if end_of_hunk {
                text.push(Text::Styled(
                    Cow::from(symbols::line::BOTTOM_LEFT),
                    style,
                ));
            } else {
                text.push(match line.line_type {
                    DiffLineType::Header => Text::Styled(
                        Cow::from(symbols::line::TOP_LEFT),
                        style,
                    ),
                    _ => Text::Styled(
                        Cow::from(symbols::line::VERTICAL),
                        style,
                    ),
                });
            }
        }

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

        text.push(Text::Styled(
            content,
            theme.diff_line(line.line_type, selected),
        ));
    }

    fn hunk_visible(
        hunk_min: u16,
        hunk_max: u16,
        min: u16,
        max: u16,
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

    fn add_hunk(&self) {
        if let Some(hunk) = self.selected_hunk {
            let hash = self.diff.hunks
                [usize::try_from(hunk).unwrap()]
            .header_hash;
            self.queue
                .borrow_mut()
                .push_back(InternalEvent::AddHunk(hash));
        }
    }
}

impl DrawableComponent for DiffComponent {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, r: Rect) {
        self.current_height = r.height.saturating_sub(2);
        let title =
            format!("{}{}", strings::TITLE_DIFF, self.current.path);
        f.render_widget(
            Paragraph::new(self.get_text(r.width, r.height).iter())
                .block(
                    Block::default()
                        .title(title.as_str())
                        .borders(Borders::ALL)
                        .border_style(self.theme.block(self.focused))
                        .title_style(
                            Style::default().modifier(Modifier::BOLD),
                        ),
                )
                .alignment(Alignment::Left),
            r,
        );
    }
}

impl Component for DiffComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        _force_all: bool,
    ) -> CommandBlocking {
        out.push(CommandInfo::new(
            commands::SCROLL,
            self.can_scroll(),
            self.focused,
        ));

        out.push(
            CommandInfo::new(
                commands::DIFF_HOME_END,
                self.can_scroll(),
                self.focused,
            )
            .hidden(),
        );

        out.push(CommandInfo::new(
            commands::DIFF_HUNK_REMOVE,
            self.selected_hunk.is_some(),
            self.focused && self.current.is_stage,
        ));
        out.push(CommandInfo::new(
            commands::DIFF_HUNK_ADD,
            self.selected_hunk.is_some(),
            self.focused && !self.current.is_stage,
        ));

        CommandBlocking::PassingOn
    }

    fn event(&mut self, ev: Event) -> bool {
        if self.focused {
            if let Event::Key(e) = ev {
                return match e {
                    keys::MOVE_DOWN => {
                        self.scroll(ScrollType::Down);
                        true
                    }
                    keys::SHIFT_DOWN | keys::END => {
                        self.scroll(ScrollType::End);
                        true
                    }
                    keys::HOME | keys::SHIFT_UP => {
                        self.scroll(ScrollType::Home);
                        true
                    }
                    keys::MOVE_UP => {
                        self.scroll(ScrollType::Up);
                        true
                    }
                    keys::PAGE_UP => {
                        self.scroll(ScrollType::PageUp);
                        true
                    }
                    keys::PAGE_DOWN => {
                        self.scroll(ScrollType::PageDown);
                        true
                    }
                    keys::ENTER => {
                        self.add_hunk();
                        true
                    }
                    _ => false,
                };
            }
        }

        false
    }

    fn focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self, focus: bool) {
        self.focused = focus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lineendings() {
        let mut text = Vec::new();
        DiffComponent::add_line(
            &mut text,
            10,
            &DiffLine {
                content: String::from("line 1\r\n"),
                line_type: DiffLineType::None,
            },
            false,
            false,
            false,
            Theme::empty(),
        );

        assert_eq!(text.len(), 2);

        if let Text::Styled(c, _) = &text[1] {
            assert_eq!(c, "line 1\n");
        } else {
            panic!("err")
        }
    }
}
