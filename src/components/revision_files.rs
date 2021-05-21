use std::{
    cell::Cell, collections::BTreeSet, convert::From, path::Path,
};

use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent, EventState,
};
use crate::{
    keys::SharedKeyConfig,
    queue::{InternalEvent, Queue},
    strings::{self, order},
    ui::{self, style::SharedTheme},
};
use anyhow::Result;
use asyncgit::{
    sync::{self, CommitId, TreeFile},
    AsyncNotification, CWD,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use filetree::{FileTree, MoveSelection};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

const FOLDER_ICON_COLLAPSED: &str = "\u{25b8}"; //▸
const FOLDER_ICON_EXPANDED: &str = "\u{25be}"; //▾
const EMPTY_STR: &str = "";

pub struct RevisionFilesComponent {
    queue: Queue,
    title: String,
    theme: SharedTheme,
    files: Vec<TreeFile>,
    current_file: Option<(String, String)>,
    tree: FileTree,
    scroll_top: Cell<usize>,
    revision: Option<CommitId>,
    visible: bool,
    key_config: SharedKeyConfig,
}

impl RevisionFilesComponent {
    ///
    pub fn new(
        queue: &Queue,
        _sender: &Sender<AsyncNotification>,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            queue: queue.clone(),
            title: String::new(),
            tree: FileTree::default(),
            theme,
            scroll_top: Cell::new(0),
            current_file: None,
            files: Vec::new(),
            revision: None,
            visible: false,
            key_config,
        }
    }

    ///
    pub fn open(&mut self, commit: CommitId) -> Result<()> {
        self.files = sync::tree_files(CWD, commit)?;
        let filenames: Vec<&str> = self
            .files
            .iter()
            .map(|f| f.path.to_str().unwrap_or_default())
            .collect();
        self.tree = FileTree::new(&filenames, &BTreeSet::new())?;
        self.tree.collapse_but_root();
        self.revision = Some(commit);
        self.title = format!(
            "File Tree at [{}]",
            self.revision
                .map(|c| c.get_short_string())
                .unwrap_or_default()
        );
        self.show()?;

        Ok(())
    }

    fn tree_item_to_span<'a>(
        item: &'a filetree::FileTreeItem,
        theme: &SharedTheme,
        selected: bool,
    ) -> Span<'a> {
        let path = item.info().path();
        let indent = item.info().indent();

        let indent_str = if indent == 0 {
            String::from("")
        } else {
            format!("{:w$}", " ", w = (indent as usize) * 2)
        };

        let is_path = item.kind().is_path();
        let path_arrow = if is_path {
            if item.kind().is_path_collapsed() {
                FOLDER_ICON_COLLAPSED
            } else {
                FOLDER_ICON_EXPANDED
            }
        } else {
            EMPTY_STR
        };

        let path = format!("{}{}{}", indent_str, path_arrow, path);
        Span::styled(path, theme.file_tree_item(is_path, selected))
    }

    fn blame(&self) -> bool {
        self.tree.selected_file().map_or(false, |file| {
            self.queue.borrow_mut().push_back(
                InternalEvent::BlameFile(
                    file.full_path()
                        .strip_prefix("./")
                        .unwrap_or_default()
                        .to_string(),
                ),
            );
            true
        })
    }

    fn selection_changed(&mut self) -> Result<()> {
        if let Some(file) = self.tree.selected_file().map(|file| {
            file.full_path()
                .strip_prefix("./")
                .unwrap_or_default()
                .to_string()
        }) {
            let already_loaded = self
                .current_file
                .as_ref()
                .map(|(current_file, _)| current_file == &file)
                .unwrap_or_default();

            if !already_loaded {
                self.load_file(file)?;
            }
        } else {
            self.current_file = None;
        }

        Ok(())
    }

    fn load_file(&mut self, path: String) -> Result<()> {
        if let Some(item) = self
            .files
            .iter()
            .find(|f| f.path.ends_with(Path::new(&path)))
        {
            let content = sync::tree_file_content(CWD, item)?;
            self.current_file = Some((path, content));
        }

        Ok(())
    }
}

impl DrawableComponent for RevisionFilesComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        area: Rect,
    ) -> Result<()> {
        if self.is_visible() {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints(
                    [
                        Constraint::Percentage(40),
                        Constraint::Percentage(60),
                    ]
                    .as_ref(),
                )
                .split(area);

            let tree_height = usize::from(chunks[0].height);

            let selection = self.tree.visual_selection();
            selection.map_or_else(
                || self.scroll_top.set(0),
                |selection| {
                    self.scroll_top.set(ui::calc_scroll_top(
                        self.scroll_top.get(),
                        tree_height,
                        selection.index,
                    ))
                },
            );

            let items = self
                .tree
                .iterate(self.scroll_top.get(), tree_height)
                .map(|(item, selected)| {
                    Self::tree_item_to_span(
                        item,
                        &self.theme,
                        selected,
                    )
                });

            f.render_widget(Clear, area);
            f.render_widget(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        &self.title,
                        self.theme.title(true),
                    ))
                    .border_style(self.theme.block(true)),
                area,
            );

            ui::draw_list_block(
                f,
                chunks[0],
                Block::default()
                    .borders(Borders::RIGHT)
                    .border_style(self.theme.block(true)),
                items,
            );

            let content = Paragraph::new(Text::from(
                self.current_file
                    .as_ref()
                    .map(|(_, content)| content.as_str())
                    .unwrap_or_default(),
            ))
            .wrap(Wrap { trim: false });
            f.render_widget(content, chunks[1]);
        }

        Ok(())
    }
}

impl Component for RevisionFilesComponent {
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
                    strings::commands::blame_file(&self.key_config),
                    self.tree.selected_file().is_some(),
                    true,
                )
                .order(order::NAV),
            );

            tree_nav_cmds(&self.tree, &self.key_config, out);
        }

        visibility_blocking(self)
    }

    fn event(
        &mut self,
        event: crossterm::event::Event,
    ) -> Result<EventState> {
        if self.is_visible() {
            if let Event::Key(key) = event {
                let consumed = if key == self.key_config.exit_popup {
                    self.hide();
                    true
                } else if key == self.key_config.blame {
                    if self.blame() {
                        self.hide();
                        true
                    } else {
                        false
                    }
                } else if tree_nav(
                    &mut self.tree,
                    &self.key_config,
                    key,
                ) {
                    self.selection_changed()?;
                    true
                } else {
                    false
                };

                return Ok(consumed.into());
            }
        }

        Ok(EventState::NotConsumed)
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

//TODO: reuse for other tree usages
fn tree_nav_cmds(
    tree: &FileTree,
    key_config: &SharedKeyConfig,
    out: &mut Vec<CommandInfo>,
) {
    out.push(
        CommandInfo::new(
            strings::commands::navigate_tree(key_config),
            !tree.is_empty(),
            true,
        )
        .order(order::NAV),
    );
}

//TODO: reuse for other tree usages
fn tree_nav(
    tree: &mut FileTree,
    key_config: &SharedKeyConfig,
    key: crossterm::event::KeyEvent,
) -> bool {
    if key == key_config.move_down {
        tree.move_selection(MoveSelection::Down)
    } else if key == key_config.move_up {
        tree.move_selection(MoveSelection::Up)
    } else if key == key_config.move_right {
        tree.move_selection(MoveSelection::Right)
    } else if key == key_config.move_left {
        tree.move_selection(MoveSelection::Left)
    } else if key == key_config.home || key == key_config.shift_up {
        tree.move_selection(MoveSelection::Top)
    } else if key == key_config.end || key == key_config.shift_down {
        tree.move_selection(MoveSelection::End)
    } else if key == key_config.tree_collapse_recursive {
        tree.collapse_recursive();
        true
    } else if key == key_config.tree_expand_recursive {
        tree.expand_recursive();
        true
    } else {
        false
    }
}
