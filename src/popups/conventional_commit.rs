use std::borrow::Cow;

use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use itertools::Itertools;
use ratatui::text::Line;
use ratatui::Frame;
use ratatui::{
	layout::{Constraint, Direction, Layout, Margin, Rect},
	text::Span,
	widgets::{Block, Borders, Clear},
};
use strum::{Display, EnumIter, IntoEnumIterator};
use unicode_segmentation::UnicodeSegmentation;

use crate::components::visibility_blocking;
use crate::queue::Queue;
use crate::string_utils::trim_length_left;
use crate::ui::style::SharedTheme;
use crate::{
	app::Environment,
	components::{
		CommandBlocking, CommandInfo, Component, DrawableComponent,
		EventState, InputType, ScrollType, TextInputComponent,
	},
	keys::{key_match, SharedKeyConfig},
	ui,
};

#[derive(EnumIter, Display, Clone)]
#[strum(serialize_all = "lowercase")]
enum CommitType {
	Refactor,
	#[strum(to_string = "feat")]
	Feature,
	Fix,
	Docs,
	Style,
	#[strum(to_string = "perf")]
	Performance,
	Test,
	Build,
	CI,
	Chore,
	Revert,
	Initial,
	Bump,
	Wip,
	Debug,
}

enum MoreInfoCommit {
	// 🎨
	CodeStyle,
	// ⚡️
	Performance,
	// 🐛
	Bug,
	// 🚑️
	CriticalBug,
	// ✨
	Feature,
	// 📝
	Documentation,
	// 💄
	UI,
	// 🎉
	Initial,
	// ✅
	TestsPassing,
	// ➕
	Add,
	// ➖
	Remove,
	// 🔒️
	Security,
	// 🔖
	Release,
	// ⚠️
	Warning,
	// 🚧
	Wip,
	// ⬇️
	Down,
	// ⬆️
	Up,
	// 👷
	CI,
	// ♻️
	Refactor,
	// 📈
	TrackCode,
	// ✏️
	Typo,
	// 🌐
	Internationalization,
	// ⏪️
	Revert,
	// 📦️
	Package,
	// 👽️
	ExternalDependencyChange,
	// 🚚
	RenameResources,
	// ♿️
	Accessibility,
	// 📜
	Readme,
	// ⚖️
	License,
	// 💬
	TextLiteral,
	// ⛃
	DatabaseRelated,
	// 🔊
	AddLogs,
	// 🔇
	RemoveLogs,
	// 🚸
	ImproveExperience,
	// 🏗️
	ArchitecturalChanges,
	// 🤡
	WrittingReallyBadCode,
	// 🙈
	GitIgnore,
	// ⚗️
	Experimentations,
	// 🚩
	Flag,
	// 🗑️
	Trash,
	// 🛂
	Authorization,
	// 🩹
	QuickFix,
	// ⚰️
	RemoveDeadCode,
	// 👔
	Business,
	// 🩺
	HealthCheck,
	// 🧱
	Infra,
	// 🦺
	Validation,
}

impl MoreInfoCommit {
	fn strings(&self) -> (&'static str, &'static str, &'static str) {
		match *self {
			MoreInfoCommit::UI => ("💄", "UI", "UI related"),
			MoreInfoCommit::CodeStyle => {
				("🎨", "style", "Style of the code")
			}
			MoreInfoCommit::Performance => ("⚡️", "", "Performance"),
			MoreInfoCommit::Bug => ("🐛", "bug", "Normal bug"),
			MoreInfoCommit::CriticalBug => {
				("🚑️", "critical bug", "Critical Bug")
			}
			MoreInfoCommit::Feature => ("✨", "", "Feature"),
			MoreInfoCommit::Documentation => {
				("📝", "", "Documentation")
			}
			MoreInfoCommit::Initial => ("🎉", "", "Initial commit!"),
			MoreInfoCommit::TestsPassing => {
				("✅", "passing", "Test are now passing!")
			}
			MoreInfoCommit::Add => ("➕", "add", "Added"),
			MoreInfoCommit::Remove => ("➖", "remove", "Removed"),
			MoreInfoCommit::Security => {
				("🔒️", "security", "Secutiry related")
			}
			MoreInfoCommit::Release => {
				("🔖", "release", "A new relase")
			}
			MoreInfoCommit::Warning => ("⚠️", "warning", "Warning"),
			MoreInfoCommit::Wip => ("🚧", "", "WIP"),
			MoreInfoCommit::Down => ("⬇️", "downgrade", "Down"),
			MoreInfoCommit::Up => ("⬆️", "upgrade", "Up"),
			MoreInfoCommit::CI => ("👷", "", "CI related"),
			MoreInfoCommit::Refactor => ("♻️", "", "Refactor related"),
			MoreInfoCommit::TrackCode => {
				("📈", "track", "Tracking code")
			}
			MoreInfoCommit::Typo => ("✏️", "typo", "Typo"),
			MoreInfoCommit::Internationalization => {
				("🌐", "i18n", "Internationalization")
			}
			MoreInfoCommit::Revert => ("⏪️", "", "Revert"),
			MoreInfoCommit::Package => ("📦️", "", "Package related"),
			MoreInfoCommit::ExternalDependencyChange => (
				"👽️",
				"change due to external dep update",
				"Code related to change of ext dep",
			),
			MoreInfoCommit::RenameResources => {
				("🚚", "rename", "Rename some resources")
			}
			MoreInfoCommit::Accessibility => {
				("♿️", "accessibility", "Improved accessibility")
			}
			MoreInfoCommit::Readme => ("📜", "README", "README"),
			MoreInfoCommit::License => ("⚖️", "LICENSE", "LICENSE"),
			MoreInfoCommit::TextLiteral => {
				("💬", "raw value", "Modified literal value")
			}
			MoreInfoCommit::DatabaseRelated => {
				("⛃", "db", "Database related")
			}
			MoreInfoCommit::AddLogs => ("🔊", "add logs", "Add logs"),
			MoreInfoCommit::RemoveLogs => {
				("🔇", "remove logs", "Remove logs")
			}
			MoreInfoCommit::ImproveExperience => {
				("🚸", "experience", "Improve experience")
			}
			MoreInfoCommit::ArchitecturalChanges => {
				("🏗️", "architecture", "Architectural Changes")
			}
			MoreInfoCommit::WrittingReallyBadCode => (
				"🤡",
				"really bad code",
				"This is some REALLY bad code",
			),
			MoreInfoCommit::GitIgnore => {
				("🙈", "gitignore", "GitIgnore")
			}
			MoreInfoCommit::Experimentations => {
				("⚗️", "experimentations", "Experimentations")
			}
			MoreInfoCommit::Flag => ("🚩", "flag", "Flag"),
			MoreInfoCommit::Trash => ("🗑️", "", "Trash"),
			MoreInfoCommit::Authorization => {
				("🛂", "authorization", "Authorization")
			}
			MoreInfoCommit::QuickFix => {
				("🩹", "quick-fix", "QuickFix")
			}
			MoreInfoCommit::RemoveDeadCode => {
				("⚰️", "remove dead code", "RemoveDeadCode")
			}
			MoreInfoCommit::Business => {
				("👔", "business", "Business related")
			}
			MoreInfoCommit::HealthCheck => {
				("🩺", "healthcheck", "HealthCheck")
			}
			MoreInfoCommit::Infra => ("🧱", "infra", "Infra"),
			MoreInfoCommit::Validation => {
				("🦺", "validation", "Validation")
			}
		}
	}
}

impl CommitType {
	fn more_info(&self) -> Vec<MoreInfoCommit> {
		match *self {
			CommitType::Fix => {
				vec![
					MoreInfoCommit::Bug,
					MoreInfoCommit::CriticalBug,
					MoreInfoCommit::Security,
					MoreInfoCommit::Warning,
					MoreInfoCommit::TrackCode,
					MoreInfoCommit::Typo,
					MoreInfoCommit::TextLiteral,
					MoreInfoCommit::ExternalDependencyChange,
					MoreInfoCommit::DatabaseRelated,
					MoreInfoCommit::Authorization,
					MoreInfoCommit::QuickFix,
					MoreInfoCommit::HealthCheck,
					MoreInfoCommit::Business,
					MoreInfoCommit::Infra,
				]
			}
			CommitType::Feature => vec![
				MoreInfoCommit::Feature,
				MoreInfoCommit::Security,
				MoreInfoCommit::TrackCode,
				MoreInfoCommit::Internationalization,
				MoreInfoCommit::Package,
				MoreInfoCommit::Accessibility,
				MoreInfoCommit::Readme,
				MoreInfoCommit::License,
				MoreInfoCommit::DatabaseRelated,
				MoreInfoCommit::Flag,
				MoreInfoCommit::Authorization,
				MoreInfoCommit::Business,
				MoreInfoCommit::Validation,
			],
			CommitType::Chore | CommitType::Refactor => vec![
				MoreInfoCommit::Security,
				MoreInfoCommit::Refactor,
				MoreInfoCommit::TrackCode,
				MoreInfoCommit::Internationalization,
				MoreInfoCommit::RenameResources,
				MoreInfoCommit::Accessibility,
				MoreInfoCommit::Readme,
				MoreInfoCommit::License,
				MoreInfoCommit::TextLiteral,
				MoreInfoCommit::DatabaseRelated,
				MoreInfoCommit::RemoveLogs,
				MoreInfoCommit::ImproveExperience,
				MoreInfoCommit::ArchitecturalChanges,
				MoreInfoCommit::GitIgnore,
				MoreInfoCommit::Flag,
				MoreInfoCommit::Trash,
				MoreInfoCommit::Authorization,
				MoreInfoCommit::RemoveDeadCode,
				MoreInfoCommit::Business,
				MoreInfoCommit::Infra,
				MoreInfoCommit::Validation,
			],
			CommitType::CI => vec![MoreInfoCommit::CI],
			CommitType::Initial => vec![MoreInfoCommit::Initial],
			CommitType::Performance => {
				vec![
					MoreInfoCommit::Performance,
					MoreInfoCommit::DatabaseRelated,
				]
			}
			CommitType::Wip => vec![
				MoreInfoCommit::Wip,
				MoreInfoCommit::WrittingReallyBadCode,
				MoreInfoCommit::Experimentations,
			],
			CommitType::Docs => vec![MoreInfoCommit::Documentation],
			CommitType::Test => vec![
				MoreInfoCommit::TestsPassing,
				MoreInfoCommit::Add,
				MoreInfoCommit::Remove,
				MoreInfoCommit::Experimentations,
				MoreInfoCommit::HealthCheck,
				MoreInfoCommit::Validation,
			],
			CommitType::Bump => {
				vec![
					MoreInfoCommit::Add,
					MoreInfoCommit::Remove,
					MoreInfoCommit::Down,
					MoreInfoCommit::Up,
					MoreInfoCommit::Release,
					MoreInfoCommit::Package,
				]
			}
			CommitType::Style => {
				vec![
					MoreInfoCommit::CodeStyle,
					MoreInfoCommit::UI,
					MoreInfoCommit::ImproveExperience,
				]
			}
			CommitType::Build => vec![MoreInfoCommit::CI],
			CommitType::Debug => vec![
				MoreInfoCommit::TrackCode,
				MoreInfoCommit::AddLogs,
				MoreInfoCommit::HealthCheck,
				MoreInfoCommit::RemoveLogs,
			],
			CommitType::Revert => vec![MoreInfoCommit::Revert],
		}
	}
}

pub struct ConventionalCommitPopup {
	key_config: SharedKeyConfig,
	is_visible: bool,
	is_insert: bool,
	query: Option<String>,
	selected_index: usize,
	options: Vec<CommitType>,
	query_results: Vec<CommitType>,
	input: TextInputComponent,
	theme: SharedTheme,
	seleted_commit_type: Option<CommitType>,
	queue: Queue,
}

impl ConventionalCommitPopup {
	///
	// pub fn new(env: &Environment) -> Self {
	pub fn new(env: &Environment) -> Self {
		let mut input =
			TextInputComponent::new(env, "", "Filter ", false)
				.with_input_type(InputType::Singleline);
		input.embed();

		Self {
			selected_index: 0,
			input,
			options: CommitType::iter().collect_vec(),
			query_results: CommitType::iter().collect_vec(),
			is_insert: false,
			query: None,
			is_visible: false,
			key_config: env.key_config.clone(),
			seleted_commit_type: None,
			theme: env.theme.clone(),
			queue: env.queue.clone(),
		}
	}

	#[inline]
	fn draw_matches_list(&self, f: &mut Frame, mut area: Rect) {
		{
			// Block has two lines up and down which need to be considered
			const HEIGHT_BLOCK_MARGIN: usize = 2;

			let title =
				format!("Results: {}", self.query_results.len());

			let height = usize::from(area.height);
			let width = usize::from(area.width);

			let list_height =
				height.saturating_sub(HEIGHT_BLOCK_MARGIN);

			let scroll_skip =
				self.selected_index.saturating_sub(list_height);
			let quick_shortcuts = self.quick_shortcuts();

			let iter_over = if let Some(commit_type) =
				&self.seleted_commit_type
			{
				commit_type
					.more_info()
					.iter()
					.enumerate()
					.take(height)
					.map(|(idx, more_info)| {
						let (emoji, _, long_name) =
							more_info.strings();
						let text_string =
							format!("{emoji} {long_name}");
						let text = trim_length_left(
							&text_string,
							width - 4, // ` [k]`
						);
						(self.selected_index == idx, text.to_owned())
					})
					.collect_vec()
			} else {
				self.query_results
					.iter()
					.enumerate()
					.take(height)
					.map(|(idx, commit_type)| {
						let commit_type_string =
							commit_type.to_string();
						let text = trim_length_left(
							commit_type_string.as_str(),
							width - 4, // ` [k]`
						);
						//FIXME: not working
						(
							self.selected_index == idx,
							format!(
								"{:w$} [{}]",
								text,
								quick_shortcuts[idx],
								w = width,
							),
						)
					})
					.collect_vec()
			};

			let items =
				iter_over.into_iter().map(|(selected, text)| {
					Line::from(
						text.graphemes(true)
							.enumerate()
							.map(|(c_idx, c)| {
								Span::styled(
									Cow::from(c.to_string()),
									self.theme
										.text(selected, selected),
								)
							})
							.collect::<Vec<_>>(),
					)
				});

			ui::draw_list_block(
				f,
				area,
				Block::default()
					.title(Span::styled(
						title,
						self.theme.title(true),
					))
					.borders(Borders::TOP),
				items,
			);
		}
	}

	pub fn quick_shortcuts(&self) -> Vec<char> {
		// Missing `i`, because `i` is mapped to insert sorry~
		let default = "qwertyuopasdfghjklmzxcvbn";

		let dont_map_keys = [
			self.key_config.keys.move_down,
			self.key_config.keys.move_up,
			self.key_config.keys.exit_popup,
			self.key_config.keys.exit,
			self.key_config.keys.insert,
		]
		.into_iter()
		.filter_map(|k| {
			if let KeyCode::Char(c) = k.code {
				Some(c)
			} else {
				None
			}
		})
		.collect_vec();

		default
			.chars()
			.filter(|c| !dont_map_keys.contains(c))
			.take(self.query_results.len())
			.collect_vec()
	}

	pub fn move_selection(&mut self, direction: ScrollType) {
		let new_selection = match direction {
			ScrollType::Up => self.selected_index.saturating_sub(1),
			ScrollType::Down => self.selected_index.saturating_add(1),
			_ => self.selected_index,
		};

		let new_selection = new_selection
			.clamp(0, self.query_results.len().saturating_sub(1));

		self.selected_index = new_selection;
	}

	pub fn any_work_pending(&self) -> bool {
		false
	}

	fn update_query(&mut self) {
		if self
			.query
			.as_ref()
			.map_or(true, |q| q != self.input.get_text())
		{
			self.set_query(self.input.get_text().to_string());
		}
	}

	fn set_query(&mut self, query: String) {
		self.query = Some(query.clone());
		self.query_results = self
			.options
			.iter()
			.filter(|option| option.to_string() == query)
			.cloned()
			.collect_vec();
	}

	fn validate_escape(&mut self, commit_type: CommitType) {
		let (emoji, short_msg, _) =
			commit_type.more_info()[self.selected_index].strings();
		self.queue.push(crate::queue::InternalEvent::OpenCommit);
		self.queue.push(
			crate::queue::InternalEvent::AddCommitMessage(format!(
				"{emoji} {commit_type}: {short_msg}"
			)),
		);
		self.hide();
		self.selected_index = 0;
		self.seleted_commit_type = None;
	}
}

impl DrawableComponent for ConventionalCommitPopup {
	fn draw(&self, f: &mut Frame, area: Rect) -> Result<()> {
		// if self.is_visible() {
		// 	self.input.draw(f, rect)?;
		// 	self.draw_warnings(f);
		// }
		//
		// Ok(())
		if self.is_visible {
			const MAX_SIZE: (u16, u16) = (50, 20);

			let area = ui::centered_rect_absolute(
				MAX_SIZE.0, MAX_SIZE.1, area,
			);

			f.render_widget(Clear, area);
			f.render_widget(
				Block::default()
					.borders(Borders::all())
					.style(self.theme.title(true))
					.title(Span::styled(
						"owo",
						self.theme.title(true),
					)),
				area,
			);

			let chunks = Layout::default()
				.direction(Direction::Vertical)
				.constraints(
					[
						Constraint::Length(1),
						Constraint::Percentage(100),
					]
					.as_ref(),
				)
				.split(area.inner(Margin {
					horizontal: 1,
					vertical: 1,
				}));

			self.input.draw(f, chunks[0])?;

			self.draw_matches_list(f, chunks[1]);
		}

		Ok(())
	}
}

impl Component for ConventionalCommitPopup {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		// if self.is_visible() || force_all {
		// 	self.input.commands(out, force_all);
		//
		// 	out.push(CommandInfo::new(
		// 		strings::commands::create_branch_confirm_msg(
		// 			&self.key_config,
		// 		),
		// 		true,
		// 		true,
		// 	));
		// }
		//
		if self.is_visible() || force_all {
			// out.push(CommandInfo::new(
			// 	strings::commands::scroll_popup(&self.key_config),
			// 	true,
			// 	true,
			// ));
			//
			// out.push(CommandInfo::new(
			// 	strings::commands::close_fuzzy_finder(
			// 		&self.key_config,
			// 	),
			// 	true,
			// 	true,
			// ));
		}

		visibility_blocking(self)
	}
	fn event(
		&mut self,
		event: &crossterm::event::Event,
	) -> Result<EventState> {
		if self.is_visible() {
			if self.is_insert {
				println!("lol");
			}
			if let Event::Key(key) = event {
				if key_match(key, self.key_config.keys.exit_popup)
					|| key_match(key, self.key_config.keys.enter)
				{
					if let Some(commit_type) =
						self.seleted_commit_type.clone()
					{
						self.validate_escape(commit_type);
					} else {
						let commit = self
							.query_results
							.get(self.selected_index)
							.cloned();

						self.seleted_commit_type = commit.clone();
						self.selected_index = 0;

						if let Some(more_infos) =
							commit.as_ref().map(|c| c.more_info())
						{
							if more_infos.len() == 1 {
								self.validate_escape(commit.unwrap());
							}
						}
					}
				} else if key_match(key, self.key_config.keys.insert)
				{
					self.is_insert = true;
				} else if key_match(
					key,
					self.key_config.keys.popup_down,
				) {
					self.move_selection(ScrollType::Down);
				} else if key_match(
					key,
					self.key_config.keys.popup_up,
				) {
					self.move_selection(ScrollType::Up);
				} else {
					if self.input.event(event)?.is_consumed() {
						self.update_query();
					}
				}
			}

			return Ok(EventState::Consumed);
		}

		Ok(EventState::NotConsumed)
	}

	fn is_visible(&self) -> bool {
		self.is_visible
	}

	fn hide(&mut self) {
		self.is_visible = false;
	}

	fn show(&mut self) -> Result<()> {
		self.is_visible = true;
		Ok(())
	}
}
