use super::{
	utils::scroll_vertical::VerticalScroll, BlameFileOpen,
	CommandBlocking, CommandInfo, Component, DrawableComponent,
	EventState, FileRevOpen, SyntaxTextComponent,
};
use crate::{
	keys::{key_match, SharedKeyConfig},
	queue::{InternalEvent, Queue, StackablePopupOpen},
	strings::{self, order, symbol},
	ui::{self, common_nav, style::SharedTheme},
	AsyncAppNotification, AsyncNotification,
};
use anyhow::Result;
use asyncgit::sync::{self, CommitId, RepoPathRef, TreeFile};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use filetreelist::{FileTree, FileTreeItem};
use std::{
	collections::BTreeSet,
	convert::From,
	path::{Path, PathBuf},
};
use tui::{
	backend::Backend,
	layout::{Constraint, Direction, Layout, Rect},
	text::Span,
	widgets::{Block, Borders},
	Frame,
};

enum Focus {
	Tree,
	File,
}

pub struct RevisionFilesComponent {
	repo: RepoPathRef,
	queue: Queue,
	theme: SharedTheme,
	//TODO: store TreeFiles in `tree`
	files: Vec<TreeFile>,
	current_file: SyntaxTextComponent,
	tree: FileTree,
	scroll: VerticalScroll,
	visible: bool,
	revision: Option<CommitId>,
	focus: Focus,
	key_config: SharedKeyConfig,
}

impl RevisionFilesComponent {
	///
	pub fn new(
		repo: RepoPathRef,
		queue: &Queue,
		sender: &Sender<AsyncAppNotification>,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			queue: queue.clone(),
			tree: FileTree::default(),
			scroll: VerticalScroll::new(),
			current_file: SyntaxTextComponent::new(
				repo.clone(),
				sender,
				key_config.clone(),
				theme.clone(),
			),
			theme,
			files: Vec::new(),
			revision: None,
			focus: Focus::Tree,
			key_config,
			repo,
			visible: false,
		}
	}

	///
	pub fn set_commit(&mut self, commit: CommitId) -> Result<()> {
		self.show()?;

		let same_id =
			self.revision.map(|c| c == commit).unwrap_or_default();
		if !same_id {
			self.files =
				sync::tree_files(&self.repo.borrow(), commit)?;
			let filenames: Vec<&Path> =
				self.files.iter().map(|f| f.path.as_path()).collect();
			self.tree = FileTree::new(&filenames, &BTreeSet::new())?;
			self.tree.collapse_but_root();
			self.revision = Some(commit);
		}

		Ok(())
	}

	///
	pub const fn revision(&self) -> Option<CommitId> {
		self.revision
	}

	///
	pub const fn selection(&self) -> Option<usize> {
		self.tree.selection()
	}

	///
	pub fn update(&mut self, ev: AsyncNotification) {
		self.current_file.update(ev);
	}

	///
	pub fn any_work_pending(&self) -> bool {
		self.current_file.any_work_pending()
	}

	fn tree_item_to_span<'a>(
		item: &'a FileTreeItem,
		theme: &SharedTheme,
		width: usize,
		selected: bool,
	) -> Span<'a> {
		let path = item.info().path_str();
		let indent = item.info().indent();

		let indent_str = if indent == 0 {
			String::from("")
		} else {
			format!("{:w$}", " ", w = (indent as usize) * 2)
		};

		let is_path = item.kind().is_path();
		let path_arrow = if is_path {
			if item.kind().is_path_collapsed() {
				symbol::FOLDER_ICON_COLLAPSED
			} else {
				symbol::FOLDER_ICON_EXPANDED
			}
		} else {
			symbol::EMPTY_STR
		};

		let available_width =
			width.saturating_sub(indent_str.len() + path_arrow.len());

		let path = format!(
			"{}{}{:w$}",
			indent_str,
			path_arrow,
			path,
			w = available_width
		);

		Span::styled(path, theme.file_tree_item(is_path, selected))
	}

	fn blame(&self) -> bool {
		self.selected_file_path().map_or(false, |path| {
			self.queue.push(InternalEvent::OpenPopup(
				StackablePopupOpen::BlameFile(BlameFileOpen {
					file_path: path,
					commit_id: self.revision,
					selection: None,
				}),
			));

			true
		})
	}

	fn file_history(&self) -> bool {
		self.selected_file_path().map_or(false, |path| {
			self.queue.push(InternalEvent::OpenPopup(
				StackablePopupOpen::FileRevlog(FileRevOpen::new(
					path,
				)),
			));

			true
		})
	}

	fn open_finder(&self) {
		self.queue
			.push(InternalEvent::OpenFileFinder(self.files.clone()));
	}

	pub fn find_file(&mut self, file: &Option<PathBuf>) {
		if let Some(file) = file {
			self.tree.collapse_but_root();
			if self.tree.select_file(file) {
				self.selection_changed();
			}
		}
	}

	fn selected_file_path_with_prefix(&self) -> Option<String> {
		self.tree
			.selected_file()
			.map(|file| file.full_path_str().to_string())
	}

	fn selected_file_path(&self) -> Option<String> {
		self.tree.selected_file().map(|file| {
			file.full_path_str()
				.strip_prefix("./")
				.unwrap_or_default()
				.to_string()
		})
	}

	fn selection_changed(&mut self) {
		//TODO: retrieve TreeFile from tree datastructure
		if let Some(file) = self.selected_file_path_with_prefix() {
			log::info!("selected: {:?}", file);
			let path = Path::new(&file);
			if let Some(item) =
				self.files.iter().find(|f| f.path == path)
			{
				if let Ok(path) = path.strip_prefix("./") {
					return self.current_file.load_file(
						path.to_string_lossy().to_string(),
						item,
					);
				}
			}
			self.current_file.clear();
		}
	}

	fn draw_tree<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
		let tree_height = usize::from(area.height.saturating_sub(2));
		let tree_width = usize::from(area.width);

		self.tree.visual_selection().map_or_else(
			|| {
				self.scroll.reset();
			},
			|selection| {
				self.scroll.update(
					selection.index,
					selection.count,
					tree_height,
				);
			},
		);

		let items = self
			.tree
			.iterate(self.scroll.get_top(), tree_height)
			.map(|(item, selected)| {
				Self::tree_item_to_span(
					item,
					&self.theme,
					tree_width,
					selected,
				)
			});

		let is_tree_focused = matches!(self.focus, Focus::Tree);

		let title = format!(
			"Files at [{}]",
			self.revision
				.map(|c| c.get_short_string())
				.unwrap_or_default(),
		);
		ui::draw_list_block(
			f,
			area,
			Block::default()
				.title(Span::styled(
					title,
					self.theme.title(is_tree_focused),
				))
				.borders(Borders::ALL)
				.border_style(self.theme.block(is_tree_focused)),
			items,
		);

		if is_tree_focused {
			self.scroll.draw(f, area, &self.theme);
		}
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
				.constraints(
					[
						Constraint::Percentage(40),
						Constraint::Percentage(60),
					]
					.as_ref(),
				)
				.split(area);

			self.draw_tree(f, chunks[0]);

			self.current_file.draw(f, chunks[1])?;
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
		if !self.is_visible() && !force_all {
			return CommandBlocking::PassingOn;
		}

		let is_tree_focused = matches!(self.focus, Focus::Tree);

		if is_tree_focused || force_all {
			out.push(
				CommandInfo::new(
					strings::commands::blame_file(&self.key_config),
					self.tree.selected_file().is_some(),
					true,
				)
				.order(order::NAV),
			);
			out.push(CommandInfo::new(
				strings::commands::edit_item(&self.key_config),
				self.tree.selected_file().is_some(),
				true,
			));
			out.push(
				CommandInfo::new(
					strings::commands::open_file_history(
						&self.key_config,
					),
					self.tree.selected_file().is_some(),
					true,
				)
				.order(order::RARE_ACTION),
			);
			tree_nav_cmds(&self.tree, &self.key_config, out);
		} else {
			self.current_file.commands(out, force_all);
		}

		CommandBlocking::PassingOn
	}

	fn event(
		&mut self,
		event: &crossterm::event::Event,
	) -> Result<EventState> {
		if !self.is_visible() {
			return Ok(EventState::NotConsumed);
		}

		if let Event::Key(key) = event {
			let is_tree_focused = matches!(self.focus, Focus::Tree);
			if is_tree_focused
				&& tree_nav(&mut self.tree, &self.key_config, key)
			{
				self.selection_changed();
				return Ok(EventState::Consumed);
			} else if key_match(key, self.key_config.keys.blame) {
				if self.blame() {
					self.hide();
					return Ok(EventState::Consumed);
				}
			} else if key_match(
				key,
				self.key_config.keys.file_history,
			) {
				if self.file_history() {
					self.hide();
					return Ok(EventState::Consumed);
				}
			} else if key_match(key, self.key_config.keys.move_right)
			{
				if is_tree_focused {
					self.focus = Focus::File;
					self.current_file.focus(true);
					self.focus(true);
					return Ok(EventState::Consumed);
				}
			} else if key_match(key, self.key_config.keys.move_left) {
				if !is_tree_focused {
					self.focus = Focus::Tree;
					self.current_file.focus(false);
					self.focus(false);
					return Ok(EventState::Consumed);
				}
			} else if key_match(key, self.key_config.keys.file_find) {
				if is_tree_focused {
					self.open_finder();
					return Ok(EventState::Consumed);
				}
			} else if key_match(key, self.key_config.keys.edit_file) {
				if let Some(file) =
					self.selected_file_path_with_prefix()
				{
					//Note: switch to status tab so its clear we are
					// not altering a file inside a revision here
					self.queue.push(InternalEvent::TabSwitchStatus);
					self.queue.push(
						InternalEvent::OpenExternalEditor(Some(file)),
					);
					return Ok(EventState::Consumed);
				}
			} else if !is_tree_focused {
				return self.current_file.event(event);
			}
		}

		Ok(EventState::NotConsumed)
	}

	fn hide(&mut self) {
		self.visible = false;
	}

	fn is_visible(&self) -> bool {
		self.visible
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
	key: &crossterm::event::KeyEvent,
) -> bool {
	if let Some(common_nav) = common_nav(key, key_config) {
		tree.move_selection(common_nav)
	} else if key_match(key, key_config.keys.tree_collapse_recursive)
	{
		tree.collapse_recursive();
		true
	} else if key_match(key, key_config.keys.tree_expand_recursive) {
		tree.expand_recursive();
		true
	} else {
		false
	}
}
