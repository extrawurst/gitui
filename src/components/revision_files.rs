use std::{cell::Cell, collections::BTreeSet, convert::From};

use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent, EventState,
};
use crate::{
    keys::SharedKeyConfig,
    queue::Queue,
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
    backend::Backend, layout::Rect, text::Span, widgets::Clear, Frame,
};

const FOLDER_ICON_COLLAPSED: &str = "\u{25b8}"; //▸
const FOLDER_ICON_EXPANDED: &str = "\u{25be}"; //▾
const EMPTY_STR: &str = "";

pub struct RevisionFilesComponent {
    title: String,
    theme: SharedTheme,
    files: Vec<TreeFile>,
    tree: FileTree,
    scroll_top: Cell<usize>,
    revision: Option<CommitId>,
    visible: bool,
    key_config: SharedKeyConfig,
    current_height: std::cell::Cell<usize>,
}

impl RevisionFilesComponent {
    ///
    pub fn new(
        _queue: &Queue,
        _sender: &Sender<AsyncNotification>,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            title: String::new(),
            tree: FileTree::default(),
            theme,
            scroll_top: Cell::new(0),
            files: Vec::new(),
            revision: None,
            visible: false,
            key_config,
            current_height: std::cell::Cell::new(0),
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
}

impl DrawableComponent for RevisionFilesComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        area: Rect,
    ) -> Result<()> {
        if self.is_visible() {
            let tree_height =
                usize::from(area.height.saturating_sub(2));

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
            ui::draw_list(
                f,
                area,
                &self.title,
                items,
                true,
                &self.theme,
            );

            self.current_height.set(area.height.into());
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
                } else {
                    tree_nav(&mut self.tree, &self.key_config, key)
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
