use crate::{components::CommandInfo, strings, ui::style::Theme};
use std::borrow::Cow;
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    widgets::{Paragraph, Text},
    Frame,
};
use unicode_width::UnicodeWidthStr;

enum DrawListEntry {
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
    draw_list: Vec<DrawListEntry>,
    cmd_infos: Vec<CommandInfo>,
    theme: Theme,
    lines: u16,
    width: u16,
}

impl CommandBar {
    pub const fn new(theme: &Theme) -> Self {
        Self {
            draw_list: Vec::new(),
            cmd_infos: Vec::new(),
            theme: *theme,
            lines: 0,
            width: 0,
        }
    }

    pub fn refresh_width(&mut self, width: u16) {
        if width != self.width {
            self.refresh_list(width);
            self.width = width;
        }
    }

    fn refresh_list(&mut self, width: u16) {
        self.draw_list.clear();

        let mut line_width = 0_usize;
        let mut lines = 1_u16;

        for c in &self.cmd_infos {
            let entry_w = UnicodeWidthStr::width(c.text.name);

            if line_width + entry_w > width as usize {
                self.draw_list.push(DrawListEntry::LineBreak);
                line_width = 0;
                lines += 1;
            } else if line_width > 0 {
                self.draw_list.push(DrawListEntry::Splitter);
            }

            line_width += entry_w + 1;

            self.draw_list.push(DrawListEntry::Command(Command {
                txt: c.text.name.to_string(),
                enabled: c.enabled,
                line: lines.saturating_sub(1) as usize,
            }));
        }

        self.lines = lines;
    }

    pub fn set_cmds(&mut self, cmds: Vec<CommandInfo>) {
        self.cmd_infos = cmds
            .into_iter()
            .filter(CommandInfo::show_in_quickbar)
            .collect::<Vec<_>>();
        self.cmd_infos.sort_by_key(|e| e.order);
        self.refresh_list(self.width);
    }

    pub const fn height(&self) -> u16 {
        self.lines
    }

    pub fn draw<B: Backend>(&self, f: &mut Frame<B>, r: Rect) {
        let splitter = Text::Raw(Cow::from(strings::CMD_SPLITTER));

        let texts = self
            .draw_list
            .iter()
            .map(|c| match c {
                DrawListEntry::Command(c) => Text::Styled(
                    Cow::from(c.txt.as_str()),
                    self.theme.commandbar(c.enabled, c.line),
                ),
                DrawListEntry::LineBreak => {
                    Text::Raw(Cow::from("\n"))
                }
                DrawListEntry::Splitter => splitter.clone(),
            })
            .collect::<Vec<_>>();

        f.render_widget(
            Paragraph::new(texts.iter()).alignment(Alignment::Left),
            r,
        );
    }
}
