use crate::{components::CommandInfo, strings, ui::style::Theme};
use std::borrow::Cow;
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    widgets::{Paragraph, Text},
    Frame,
};
use unicode_width::UnicodeWidthStr;

enum CommandEntry {
    LineBreak,
    Splitter,
    Command(Command),
}

struct Command {
    txt: String,
    enabled: bool,
    line: usize,
}

/// helper to be used while drawing
pub struct CommandBar {
    cmds: Vec<CommandEntry>,
    theme: Theme,
    lines: u16,
}

impl CommandBar {
    pub fn new(
        cmds: &[CommandInfo],
        theme: &Theme,
        width: u16,
    ) -> Self {
        let mut cmdlist = Vec::with_capacity(cmds.len());

        let mut line_width = 0_usize;
        let mut lines = 1_u16;
        for c in cmds {
            if c.show_in_quickbar() {
                let entry_w = UnicodeWidthStr::width(c.text.name);

                if line_width + entry_w + 1 > width as usize {
                    cmdlist.push(CommandEntry::LineBreak);
                    line_width = 0;
                    lines += 1;
                } else if line_width > 0 {
                    cmdlist.push(CommandEntry::Splitter);
                }

                line_width += entry_w + 1;

                cmdlist.push(CommandEntry::Command(Command {
                    txt: c.text.name.to_string(),
                    enabled: c.enabled,
                    line: lines.saturating_sub(1) as usize,
                }));
            }
        }

        Self {
            cmds: cmdlist,
            theme: *theme,
            lines,
        }
    }

    pub const fn height(&self) -> u16 {
        self.lines
    }

    pub fn draw<B: Backend>(&self, f: &mut Frame<B>, r: Rect) {
        let splitter = Text::Raw(Cow::from(strings::CMD_SPLITTER));

        let texts = self
            .cmds
            .iter()
            .map(|c| match c {
                CommandEntry::Command(c) => Text::Styled(
                    Cow::from(c.txt.as_str()),
                    self.theme.commandbar(c.enabled, c.line),
                ),
                CommandEntry::LineBreak => Text::Raw(Cow::from("\n")),
                CommandEntry::Splitter => splitter.clone(),
            })
            .collect::<Vec<_>>();

        f.render_widget(
            Paragraph::new(texts.iter()).alignment(Alignment::Left),
            r,
        );
    }
}
