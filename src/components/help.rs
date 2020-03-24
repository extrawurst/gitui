use super::{CommandInfo, Component};
use crate::{strings, ui};
use crossterm::event::{Event, KeyCode};
use std::borrow::Cow;
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph, Text, Widget},
    Frame,
};

#[derive(Default)]
pub struct HelpComponent {
    cmds: Vec<CommandInfo>,
    visible: bool,
}

impl Component for HelpComponent {
    fn draw<B: Backend>(&self, f: &mut Frame<B>, _rect: Rect) {
        if self.visible {
            let txt = self
                .cmds
                .iter()
                .map(|e| {
                    let mut out = String::new();
                    e.print(&mut out);
                    out.push('\n');
                    Text::Raw(Cow::from(out))
                })
                .collect::<Vec<_>>();

            ui::Clear::new(
                Paragraph::new(txt.iter())
                    .block(
                        Block::default()
                            .title(strings::HELP_TITLE)
                            .borders(Borders::ALL),
                    )
                    .alignment(Alignment::Left),
            )
            .render(f, ui::centered_rect_absolute(60, 20, f.size()));
        }
    }

    fn commands(&self) -> Vec<CommandInfo> {
        vec![CommandInfo::new(
            strings::COMMIT_CMD_CLOSE,
            true,
            self.visible,
        )]
    }

    fn event(&mut self, ev: Event) -> bool {
        if let Event::Key(e) = ev {
            return match e.code {
                KeyCode::Esc => {
                    self.hide();
                    true
                }
                _ => false,
            };
        }

        false
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hide(&mut self) {
        self.visible = false
    }

    fn show(&mut self) {
        self.visible = true
    }
}

impl HelpComponent {
    ///
    pub fn set_cmds(&mut self, cmds: Vec<CommandInfo>) {
        self.cmds = cmds;
    }
}
