use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent, EventUpdate,
};
use crate::{keys, strings, ui, version::Version};
use asyncgit::hash;
use crossterm::event::Event;
use itertools::Itertools;
use std::{borrow::Cow, cmp, convert::TryFrom};
use strings::commands;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Text, Widget},
    Frame,
};

///
#[derive(Default)]
pub struct HelpComponent {
    cmds: Vec<CommandInfo>,
    visible: bool,
    selection: u16,
}

impl DrawableComponent for HelpComponent {
    fn draw<B: Backend>(&self, f: &mut Frame<B>, _rect: Rect) {
        if self.visible {
            let (txt, selected_line) = self.get_text();

            let height = 24;
            let scroll_threshold = height / 3;

            let scroll = if selected_line > scroll_threshold {
                self.selection - scroll_threshold
            } else {
                0
            };

            let area =
                ui::centered_rect_absolute(65, height, f.size());

            ui::Clear::new(
                Block::default()
                    .title(strings::HELP_TITLE)
                    .borders(Borders::ALL),
            )
            .render(f, area);

            let chunks = Layout::default()
                .vertical_margin(1)
                .horizontal_margin(1)
                .direction(Direction::Vertical)
                .constraints(
                    [Constraint::Min(1), Constraint::Length(1)]
                        .as_ref(),
                )
                .split(area);

            Paragraph::new(txt.iter())
                .scroll(scroll)
                .alignment(Alignment::Left)
                .render(f, chunks[0]);

            Paragraph::new(
                vec![Text::Raw(Cow::from(format!(
                    "gitui {}",
                    Version::new(),
                )))]
                .iter(),
            )
            .alignment(Alignment::Right)
            .render(f, chunks[1]);
        }
    }
}

impl Component for HelpComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        // only if help is open we have no other commands available
        if self.visible && !force_all {
            out.clear();
        }

        out.push(
            CommandInfo::new(
                commands::HELP_OPEN,
                true,
                !self.visible,
            )
            .order(99),
        );

        out.push(CommandInfo::new(
            commands::SCROLL,
            true,
            self.visible,
        ));

        out.push(CommandInfo::new(
            commands::CLOSE_POPUP,
            true,
            self.visible,
        ));

        visibility_blocking(self)
    }

    fn event(&mut self, ev: Event) -> Option<EventUpdate> {
        if self.visible {
            if let Event::Key(e) = ev {
                match e {
                    keys::EXIT_POPUP => self.hide(),
                    keys::MOVE_DOWN => self.move_selection(true),
                    keys::MOVE_UP => self.move_selection(false),
                    _ => (),
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
        self.cmds = cmds
            .into_iter()
            .filter(|e| !e.text.hide_help)
            .collect::<Vec<_>>();
        self.cmds.sort_by_key(|e| e.text);
        self.cmds.dedup_by_key(|e| e.text);
        self.cmds.sort_by_key(|e| hash(&e.text.group));
    }

    fn move_selection(&mut self, inc: bool) {
        let mut new_selection = self.selection;

        new_selection = if inc {
            new_selection.saturating_add(1)
        } else {
            new_selection.saturating_sub(1)
        };
        new_selection = cmp::max(new_selection, 0);

        if let Ok(max) = u16::try_from(self.cmds.len() - 1) {
            self.selection = cmp::min(new_selection, max);
        }
    }

    fn get_text<'a>(&self) -> (Vec<Text<'a>>, u16) {
        let mut txt = Vec::new();

        let mut processed = 0_u16;
        let mut selected_line = 0_u16;

        for (key, group) in
            &self.cmds.iter().group_by(|e| e.text.group)
        {
            txt.push(Text::Styled(
                Cow::from(format!(" {}\n", key)),
                Style::default().fg(Color::Black).bg(Color::Gray),
            ));

            txt.extend(
                group
                    .sorted_by_key(|e| e.order)
                    .map(|e| {
                        let is_selected = self.selection == processed;
                        if is_selected {
                            selected_line = processed;
                        }
                        processed += 1;

                        let mut out = String::from(if is_selected {
                            ">"
                        } else {
                            " "
                        });

                        e.print(&mut out);
                        out.push('\n');

                        if is_selected {
                            out.push_str(
                                format!("  {}\n", e.text.desc)
                                    .as_str(),
                            );
                        }

                        let style = if is_selected {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        };

                        Text::Styled(Cow::from(out), style)
                    })
                    .collect::<Vec<_>>(),
            );
        }

        (txt, selected_line)
    }
}
