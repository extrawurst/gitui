use crate::{
    components::{CommandInfo, Component},
    git_utils::{Diff, DiffLine, DiffLineType},
    strings,
};
use crossterm::event::{Event, KeyCode};
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Text, Widget},
    Frame,
};

///
#[derive(Default)]
pub struct DiffComponent {
    diff: Diff,
    scroll: u16,
    focused: bool,
}

impl DiffComponent {
    ///
    fn can_scroll(&self) -> bool {
        self.diff.0.len() > 1
    }
    ///
    pub fn update(&mut self, diff: Diff) {
        if diff != self.diff {
            self.diff = diff;
            self.scroll = 0;
        }
    }
    ///
    fn scroll(&mut self, inc: bool) {
        if inc {
            self.scroll =
                self.scroll.checked_add(1).unwrap_or(self.scroll);
        } else {
            self.scroll = self.scroll.checked_sub(1).unwrap_or(0);
        }
    }
}

impl Component for DiffComponent {
    fn draw<B: Backend>(&self, f: &mut Frame<B>, r: Rect) {
        let txt = self
            .diff
            .0
            .iter()
            .map(|e: &DiffLine| {
                let content = e.content.clone();
                match e.line_type {
                    DiffLineType::Delete => Text::Styled(
                        content.into(),
                        Style::default()
                            .fg(Color::Red)
                            .bg(Color::Black),
                    ),
                    DiffLineType::Add => Text::Styled(
                        content.into(),
                        Style::default()
                            .fg(Color::Green)
                            .bg(Color::Black),
                    ),
                    DiffLineType::Header => Text::Styled(
                        content.into(),
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Gray)
                            .modifier(Modifier::BOLD),
                    ),
                    _ => Text::Raw(content.into()),
                }
            })
            .collect::<Vec<_>>();

        let mut style_border = Style::default();
        let mut style_title = Style::default();
        if self.focused {
            style_border = style_border.fg(Color::Green);
            style_title = style_title.modifier(Modifier::BOLD);
        }

        Paragraph::new(txt.iter())
            .block(
                Block::default()
                    .title(strings::TITLE_DIFF)
                    .borders(Borders::ALL)
                    .border_style(style_border)
                    .title_style(style_title),
            )
            .alignment(Alignment::Left)
            .scroll(self.scroll)
            .render(f, r);
    }

    fn commands(&self) -> Vec<CommandInfo> {
        if self.focused {
            return vec![CommandInfo {
                name: strings::DIFF_CMD_SCROLL.to_string(),
                enabled: self.can_scroll(),
            }];
        }

        Vec::new()
    }

    fn event(&mut self, ev: Event) -> bool {
        if self.focused {
            if let Event::Key(e) = ev {
                // if ev == Event::Key(KeyCode::PageDown.into()) {
                //     self.scroll(true);
                // }
                // if ev == Event::Key(KeyCode::PageUp.into()) {
                //     self.scroll(false);
                // }
                // if let Event::Mouse(MouseEvent::ScrollDown(_, _, _)) = ev
                // {
                //     self.scroll(true);
                // }
                // if let Event::Mouse(MouseEvent::ScrollUp(_, _, _)) = ev {
                //     self.scroll(false);
                // }
                return match e.code {
                    KeyCode::Down => {
                        self.scroll(true);
                        true
                    }
                    KeyCode::Up => {
                        self.scroll(false);
                        true
                    }
                    _ => false,
                };
            }
        }

        false
    }

    ///
    fn focused(&self) -> bool {
        self.focused
    }
    ///
    fn focus(&mut self, focus: bool) {
        self.focused = focus
    }
}
