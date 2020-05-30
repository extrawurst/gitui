use crate::{components::CommandInfo, strings, ui::style::Theme};
use itertools::Itertools;
use std::borrow::Cow;
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Paragraph, Text},
    Frame,
};
use unicode_width::UnicodeWidthStr;

struct Command {
    txt: String,
    enabled: bool,
}

/// helper to be used while drawing
pub struct CommandBar {
    cmds: Vec<Command>,
    theme: Theme,
    max_height: u16,
}

impl CommandBar {
    pub fn new(
        cmds: &[CommandInfo],
        theme: &Theme,
        width: u16,
    ) -> Self {
        let cmds = cmds
            .iter()
            .filter_map(|c| {
                if c.show_in_quickbar() {
                    Some(Command {
                        txt: c.text.name.to_string(),
                        enabled: c.enabled,
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let text_len = cmds
            .iter()
            .map(|c| UnicodeWidthStr::width(c.txt.as_str()))
            .fold1(|a, b| a + b)
            .unwrap_or(1)
            + (cmds.len().saturating_sub(1));

        let max_height =
            if text_len > width as usize { 3 } else { 1 };

        Self {
            cmds,
            theme: *theme,
            max_height,
        }
    }

    pub const fn height(&self) -> u16 {
        self.max_height
    }

    pub fn draw<B: Backend>(&self, f: &mut Frame<B>, r: Rect) {
        let splitter = Text::Styled(
            Cow::from(strings::CMD_SPLITTER),
            Style::default(),
        );

        let texts = self
            .cmds
            .iter()
            .map(|c| {
                Text::Styled(
                    Cow::from(c.txt.as_str()),
                    self.theme.toolbar(c.enabled),
                )
            })
            .collect::<Vec<_>>();

        f.render_widget(
            Paragraph::new(texts.iter().intersperse(&splitter))
                .alignment(Alignment::Left)
                .wrap(true),
            r,
        );
    }
}
