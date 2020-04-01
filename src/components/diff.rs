use super::{DrawableComponent, EventUpdate};
use crate::{
    components::{CommandInfo, Component},
    strings,
};
use asyncgit::{hash, Diff, DiffLine, DiffLineType};
use crossterm::event::{Event, KeyCode};
use std::{borrow::Cow, cmp};
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    symbols,
    widgets::{Block, Borders, Paragraph, Text, Widget},
    Frame,
};

///
#[derive(Default)]
pub struct DiffComponent {
    diff: Diff,
    scroll: u16,
    focused: bool,
    current: (String, bool),
    current_hash: u64,
}

impl DiffComponent {
    ///
    fn can_scroll(&self) -> bool {
        self.diff.1 > 1
    }
    ///
    pub fn current(&self) -> (String, bool) {
        (self.current.0.clone(), self.current.1)
    }
    ///
    pub fn clear(&mut self) {
        self.current.0.clear();
        self.diff = Diff::default();
    }
    ///
    pub fn update(
        &mut self,
        path: String,
        is_stage: bool,
        diff: Diff,
    ) {
        let hash = hash(&diff);

        if self.current_hash != hash {
            self.current = (path, is_stage);
            self.current_hash = hash;
            self.diff = diff;
            self.scroll = 0;
        }
    }

    fn scroll(&mut self, inc: bool) {
        if inc {
            self.scroll = cmp::min(
                self.diff.1.saturating_sub(1),
                self.scroll.saturating_add(1),
            );
        } else {
            self.scroll = self.scroll.saturating_sub(1);
        }
    }

    fn get_text(&self, width: u16, height: u16) -> Vec<Text> {
        let selection = self.scroll;
        let height_d2 = height / 2;
        let min = self.scroll.saturating_sub(height_d2);
        let max = min + height;

        let mut res = Vec::new();
        let mut line_cursor = 0_u16;
        let mut lines_added = 0_u16;

        for hunk in &self.diff.0 {
            if lines_added >= height {
                break;
            }

            let hunk_len = hunk.0.len() as u16;
            let hunk_min = line_cursor;
            let hunk_max = line_cursor + hunk_len;

            if Self::hunk_visible(hunk_min, hunk_max, min, max) {
                let hunk_selected =
                    hunk_min <= selection && hunk_max > selection;
                for (i, line) in hunk.0.iter().enumerate() {
                    if line_cursor >= min {
                        Self::add_line(
                            &mut res,
                            width,
                            line,
                            selection == line_cursor,
                            hunk_selected,
                            i == hunk_len as usize - 1,
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
    ) {
        let select_color = Color::Rgb(0, 0, 100);
        let style_default = Style::default().bg(if selected {
            select_color
        } else {
            Color::Reset
        });

        {
            let style = Style::default()
                .bg(if selected || selected_hunk {
                    select_color
                } else {
                    Color::Reset
                })
                .fg(Color::DarkGray);

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

        let style_delete = Style::default()
            .fg(Color::Red)
            .bg(if selected { select_color } else { Color::Reset });
        let style_add = Style::default()
            .fg(Color::Green)
            .bg(if selected { select_color } else { Color::Reset });
        let style_header = Style::default()
            .fg(Color::Rgb(0, 0, 0))
            .bg(if selected {
                select_color
            } else {
                Color::DarkGray
            })
            .modifier(Modifier::BOLD);

        let filled = if selected {
            // selected line
            format!(
                "{:w$}\n",
                line.content.trim_matches('\n'),
                w = width as usize
            )
        } else if line.content.matches('\n').count() == 1 {
            // regular line, no selection (cheapest)
            line.content.clone()
        } else {
            // weird eof missing eol line
            format!("{}\n", line.content.trim_matches('\n'))
        };
        let content = Cow::from(filled);

        text.push(match line.line_type {
            DiffLineType::Delete => {
                Text::Styled(content, style_delete)
            }
            DiffLineType::Add => Text::Styled(content, style_add),
            DiffLineType::Header => {
                Text::Styled(content, style_header)
            }
            _ => Text::Styled(content, style_default),
        });
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
}

impl DrawableComponent for DiffComponent {
    fn draw<B: Backend>(&self, f: &mut Frame<B>, r: Rect) {
        let mut style_border = Style::default().fg(Color::DarkGray);
        let mut style_title = Style::default();
        if self.focused {
            style_border = style_border.fg(Color::Gray);
            style_title = style_title.modifier(Modifier::BOLD);
        }

        Paragraph::new(self.get_text(r.width, r.height).iter())
            .block(
                Block::default()
                    .title(strings::TITLE_DIFF)
                    .borders(Borders::ALL)
                    .border_style(style_border)
                    .title_style(style_title),
            )
            .alignment(Alignment::Left)
            .render(f, r);
    }
}

impl Component for DiffComponent {
    fn commands(&self) -> Vec<CommandInfo> {
        vec![CommandInfo::new(
            strings::CMD_SCROLL,
            self.can_scroll(),
            self.focused,
        )]
    }

    fn event(&mut self, ev: Event) -> Option<EventUpdate> {
        if self.focused {
            if let Event::Key(e) = ev {
                return match e.code {
                    KeyCode::Down => {
                        self.scroll(true);
                        Some(EventUpdate::None)
                    }
                    KeyCode::Up => {
                        self.scroll(false);
                        Some(EventUpdate::None)
                    }
                    _ => None,
                };
            }
        }

        None
    }

    fn focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self, focus: bool) {
        self.focused = focus
    }
}
