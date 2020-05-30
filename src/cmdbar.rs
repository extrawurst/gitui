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

/// helper to be used while drawing
pub struct CommandBar<'a> {
    cmds: &'a [CommandInfo],
    theme: Theme,
}

impl<'a> CommandBar<'a> {
    pub fn new(
        cmds: &'a [CommandInfo],
        theme: &Theme,
        _width: u16,
    ) -> Self {
        Self {
            cmds,
            theme: theme.clone(),
        }
    }

    pub fn height(&self) -> u16 {
        1
    }

    pub fn draw<B: Backend>(&self, f: &mut Frame<B>, r: Rect) {
        let splitter = Text::Styled(
            Cow::from(strings::CMD_SPLITTER),
            Style::default(),
        );

        let texts = self
            .cmds
            .iter()
            .filter_map(|c| {
                if c.show_in_quickbar() {
                    Some(Text::Styled(
                        Cow::from(c.text.name),
                        self.theme.toolbar(c.enabled),
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        f.render_widget(
            Paragraph::new(texts.iter().intersperse(&splitter))
                .alignment(Alignment::Left),
            r,
        );
    }
}
