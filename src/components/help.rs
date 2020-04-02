use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent, EventUpdate,
};
use crate::{keys, strings, ui};
use crossterm::event::Event;
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

impl DrawableComponent for HelpComponent {
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
}

impl Component for HelpComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
    ) -> CommandBlocking {
        // only if help is open we have no other commands available
        if self.visible {
            out.clear();
        }

        out.push(
            CommandInfo::new(
                strings::CMD_STATUS_HELP,
                true,
                !self.visible,
            )
            .order(99),
        );

        out.push(CommandInfo::new(
            strings::COMMIT_CMD_CLOSE,
            true,
            self.visible,
        ));

        visibility_blocking(self)
    }

    fn event(&mut self, ev: Event) -> Option<EventUpdate> {
        if self.visible {
            if let Event::Key(e) = ev {
                if let keys::EXIT_POPUP = e {
                    self.hide();
                }
            }

            Some(EventUpdate::Commands)
        } else if let Event::Key(keys::OPEN_HELP) = ev {
            self.show();
            Some(EventUpdate::Commands)
        } else {
            None
        }
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
