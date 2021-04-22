use super::{
    utils, visibility_blocking, CommandBlocking, CommandInfo,
    Component, DrawableComponent,
};
use crate::{
    components::{utils::string_width_align, ScrollType},
    keys::SharedKeyConfig,
    strings,
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
    sync::{blame_file, BlameHunk, FileBlame},
    CWD,
};
use crossterm::event::Event;
use std::convert::TryInto;
use tui::{
    backend::Backend,
    layout::{Constraint, Rect},
    symbols::line::VERTICAL,
    text::Span,
    widgets::{Block, Borders, Cell, Clear, Row, Table, TableState},
    Frame,
};

pub struct BlameFileComponent {
    title: String,
    theme: SharedTheme,
    visible: bool,
    path: Option<String>,
    file_blame: Option<FileBlame>,
    table_state: std::cell::Cell<TableState>,
    key_config: SharedKeyConfig,
    current_height: std::cell::Cell<usize>,
}

static COMMIT_ID: &str = "HEAD";
static NO_COMMIT_ID: &str = "0000000";
static NO_AUTHOR: &str = "<no author>";
static MIN_AUTHOR_WIDTH: usize = 3;
static MAX_AUTHOR_WIDTH: usize = 20;

fn get_author_width(width: usize) -> usize {
    (width.saturating_sub(19) / 3)
        .clamp(MIN_AUTHOR_WIDTH, MAX_AUTHOR_WIDTH)
}

const fn number_of_digits(number: usize) -> usize {
    let mut rest = number;
    let mut result = 0;

    while rest > 0 {
        rest /= 10;
        result += 1;
    }

    result
}

impl DrawableComponent for BlameFileComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        area: Rect,
    ) -> Result<()> {
        if self.is_visible() {
            let path: &str = self
                .path
                .as_deref()
                .unwrap_or("<no path for blame available>");

            let title = if self.file_blame.is_some() {
                format!("{} -- {} -- {}", self.title, path, COMMIT_ID)
            } else {
                format!(
                    "{} -- {} -- <no blame available>",
                    self.title, path
                )
            };

            let rows = self.get_rows(area.width.into());
            let author_width = get_author_width(area.width.into());
            let constraints = [
                // commit id
                Constraint::Length(7),
                // commit date
                Constraint::Length(10),
                // commit author
                Constraint::Length(author_width.try_into()?),
                // line number and vertical bar
                Constraint::Length(
                    (self.get_line_number_width().saturating_add(1))
                        .try_into()?,
                ),
                // the source code line
                Constraint::Min(0),
            ];

            let table = Table::new(rows)
                .widths(&constraints)
                .column_spacing(1)
                .highlight_style(self.theme.text(true, true))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(Span::styled(
                            title,
                            self.theme.title(true),
                        ))
                        .border_style(self.theme.block(true)),
                );

            let mut table_state = self.table_state.take();

            f.render_widget(Clear, area);
            f.render_stateful_widget(table, area, &mut table_state);

            self.table_state.set(table_state);
            self.current_height.set(area.height.into());
        }

        Ok(())
    }
}

impl Component for BlameFileComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.is_visible() || force_all {
            out.push(
                CommandInfo::new(
                    strings::commands::close_popup(&self.key_config),
                    true,
                    true,
                )
                .order(1),
            );
            out.push(
                CommandInfo::new(
                    strings::commands::scroll(&self.key_config),
                    true,
                    true,
                )
                .order(1),
            );
        }

        visibility_blocking(self)
    }

    fn event(
        &mut self,
        event: crossterm::event::Event,
    ) -> Result<bool> {
        if self.is_visible() {
            if let Event::Key(key) = event {
                if key == self.key_config.exit_popup {
                    self.hide();
                } else if key == self.key_config.move_up {
                    self.move_selection(ScrollType::Up);
                } else if key == self.key_config.move_down {
                    self.move_selection(ScrollType::Down);
                } else if key == self.key_config.shift_up
                    || key == self.key_config.home
                {
                    self.move_selection(ScrollType::Home);
                } else if key == self.key_config.shift_down
                    || key == self.key_config.end
                {
                    self.move_selection(ScrollType::End);
                } else if key == self.key_config.page_down {
                    self.move_selection(ScrollType::PageDown);
                } else if key == self.key_config.page_up {
                    self.move_selection(ScrollType::PageUp);
                }

                return Ok(true);
            }
        }

        Ok(false)
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hide(&mut self) {
        self.visible = false
    }

    fn show(&mut self) -> Result<()> {
        self.visible = true;

        Ok(())
    }
}

impl BlameFileComponent {
    ///
    pub fn new(
        title: &str,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            title: String::from(title),
            theme,
            visible: false,
            path: None,
            file_blame: None,
            table_state: std::cell::Cell::new(TableState::default()),
            key_config,
            current_height: std::cell::Cell::new(0),
        }
    }

    ///
    pub fn open(&mut self, path: &str) -> Result<()> {
        self.path = Some(path.into());
        self.file_blame = blame_file(CWD, path, COMMIT_ID).ok();
        self.table_state.get_mut().select(Some(0));

        self.show()?;

        Ok(())
    }

    ///
    fn get_rows(&self, width: usize) -> Vec<Row> {
        if let Some(ref file_blame) = self.file_blame {
            file_blame
                .lines
                .iter()
                .enumerate()
                .map(|(i, (blame_hunk, line))| {
                    self.get_line_blame(
                        width,
                        i,
                        (blame_hunk.as_ref(), line.as_ref()),
                        file_blame,
                    )
                })
                .collect()
        } else {
            vec![]
        }
    }

    fn get_line_blame(
        &self,
        width: usize,
        line_number: usize,
        hunk_and_line: (Option<&BlameHunk>, &str),
        file_blame: &FileBlame,
    ) -> Row {
        let (hunk_for_line, line) = hunk_and_line;

        let show_metadata = if line_number == 0 {
            true
        } else {
            let hunk_for_previous_line =
                &file_blame.lines[line_number - 1];

            match (hunk_for_previous_line, hunk_for_line) {
                ((Some(previous), _), Some(current)) => {
                    previous.commit_id != current.commit_id
                }
                _ => true,
            }
        };

        let mut cells = if show_metadata {
            self.get_metadata_for_line_blame(width, hunk_for_line)
        } else {
            vec![Cell::from(""), Cell::from(""), Cell::from("")]
        };

        let line_number_width = self.get_line_number_width();
        cells.push(
            Cell::from(format!(
                "{:>line_number_width$}{}",
                line_number,
                VERTICAL,
                line_number_width = line_number_width,
            ))
            .style(self.theme.text(true, false)),
        );
        cells.push(
            Cell::from(String::from(line))
                .style(self.theme.text(true, false)),
        );

        Row::new(cells)
    }

    fn get_metadata_for_line_blame(
        &self,
        width: usize,
        blame_hunk: Option<&BlameHunk>,
    ) -> Vec<Cell> {
        let commit_hash = blame_hunk.map_or_else(
            || NO_COMMIT_ID.into(),
            |hunk| hunk.commit_id.get_short_string(),
        );
        let author_width = get_author_width(width);
        let truncated_author: String = blame_hunk.map_or_else(
            || NO_AUTHOR.into(),
            |hunk| string_width_align(&hunk.author, author_width),
        );
        let author = format!(
            "{:author_width$}",
            truncated_author,
            author_width = MAX_AUTHOR_WIDTH
        );
        let time = blame_hunk.map_or_else(
            || "".into(),
            |hunk| utils::time_to_string(hunk.time, true),
        );

        vec![
            Cell::from(commit_hash)
                .style(self.theme.commit_hash(false)),
            Cell::from(time).style(self.theme.commit_time(false)),
            Cell::from(author).style(self.theme.commit_author(false)),
        ]
    }

    fn get_max_line_number(&self) -> usize {
        self.file_blame
            .as_ref()
            .map_or(0, |file_blame| file_blame.lines.len() - 1)
    }

    fn get_line_number_width(&self) -> usize {
        let max_line_number = self.get_max_line_number();

        number_of_digits(max_line_number)
    }

    fn move_selection(&mut self, scroll_type: ScrollType) -> bool {
        let mut table_state = self.table_state.take();

        let old_selection = table_state.selected().unwrap_or(0);
        let max_selection = self.get_max_line_number();

        let new_selection = match scroll_type {
            ScrollType::Up => old_selection.saturating_sub(1),
            ScrollType::Down => {
                old_selection.saturating_add(1).min(max_selection)
            }
            ScrollType::Home => 0,
            ScrollType::End => max_selection,
            ScrollType::PageUp => old_selection.saturating_sub(
                self.current_height.get().saturating_sub(2),
            ),
            ScrollType::PageDown => old_selection
                .saturating_add(
                    self.current_height.get().saturating_sub(2),
                )
                .min(max_selection),
        };

        let needs_update = new_selection != old_selection;

        table_state.select(Some(new_selection));
        self.table_state.set(table_state);

        needs_update
    }
}
