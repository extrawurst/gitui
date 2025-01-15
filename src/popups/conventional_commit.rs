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
use crate::strings;
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

#[derive(EnumIter, Display, Copy, Clone)]
#[strum(serialize_all = "lowercase")]
enum CommitType {
	Refactor,
	#[strum(to_string = "feat")]
	Feature,
	Fix,
	Wip,
	Debug,
	Test,
	Docs,
	Style,
	#[strum(to_string = "perf")]
	Performance,
	Chore,
	Revert,
	Initial,
	Bump,
	Build,
	CI,
}

#[derive(Clone)]
enum MoreInfoCommit {
	// 🎨
	CodeStyle,
	// 💅
	Formatted,
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
			MoreInfoCommit::Formatted => ("💅", "fmt", "Formatted"),
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
					MoreInfoCommit::QuickFix,
					MoreInfoCommit::Warning,
					MoreInfoCommit::Typo,
					MoreInfoCommit::TextLiteral,
					MoreInfoCommit::Security,
					MoreInfoCommit::TrackCode,
					MoreInfoCommit::ExternalDependencyChange,
					MoreInfoCommit::DatabaseRelated,
					MoreInfoCommit::Authorization,
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
				MoreInfoCommit::Refactor,
				MoreInfoCommit::ArchitecturalChanges,
				MoreInfoCommit::RenameResources,
				MoreInfoCommit::RemoveLogs,
				MoreInfoCommit::TextLiteral,
				MoreInfoCommit::RemoveDeadCode,
				MoreInfoCommit::DatabaseRelated,
				MoreInfoCommit::Security,
				MoreInfoCommit::Readme,
				MoreInfoCommit::License,
				MoreInfoCommit::ImproveExperience,
				MoreInfoCommit::TrackCode,
				MoreInfoCommit::Internationalization,
				MoreInfoCommit::Accessibility,
				MoreInfoCommit::GitIgnore,
				MoreInfoCommit::Flag,
				MoreInfoCommit::Trash,
				MoreInfoCommit::Authorization,
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
					MoreInfoCommit::Formatted,
					MoreInfoCommit::CodeStyle,
					MoreInfoCommit::UI,
					MoreInfoCommit::ImproveExperience,
				]
			}
			CommitType::Build => vec![MoreInfoCommit::CI],
			CommitType::Debug => vec![
				MoreInfoCommit::AddLogs,
				MoreInfoCommit::TrackCode,
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
	is_breaking: bool,
	query: Option<String>,
	selected_index: usize,
	options: Vec<CommitType>,
	query_results_type: Vec<CommitType>,
	query_results_more_info: Vec<MoreInfoCommit>,
	input: TextInputComponent,
	theme: SharedTheme,
	seleted_commit_type: Option<CommitType>,
	queue: Queue,
}

impl ConventionalCommitPopup {
	pub fn new(env: &Environment) -> Self {
		let mut input =
			TextInputComponent::new(env, "", "Filter ", false)
				.with_input_type(InputType::Singleline);
		input.embed();

		Self {
			selected_index: 0,
			input,
			options: CommitType::iter().collect_vec(),
			query_results_type: CommitType::iter().collect_vec(),
			query_results_more_info: Vec::new(),
			is_insert: false,
			is_breaking: false,
			query: None,
			is_visible: false,
			key_config: env.key_config.clone(),
			seleted_commit_type: None,
			theme: env.theme.clone(),
			queue: env.queue.clone(),
		}
	}

	#[inline]
	fn draw_matches_list(&self, f: &mut Frame, area: Rect) {
		let height = usize::from(area.height);
		let width = usize::from(area.width);

		let quick_shortcuts = self.quick_shortcuts();

		let title = format!(
			"Results: {}",
			if self.seleted_commit_type.is_some() {
				self.query_results_more_info.len()
			} else {
				self.query_results_type.len()
			}
		);

		let iter_over = if self.seleted_commit_type.is_some() {
			self.query_results_more_info
				.iter()
				.enumerate()
				.take(height)
				.map(|(idx, more_info)| {
					let (emoji, _, long_name) = more_info.strings();
					let text_string = format!("{emoji} {long_name}");
					let text = trim_length_left(&text_string, width);
					(
						self.selected_index == idx,
						format!("{text}{:width$}", " "),
					)
				})
				.collect_vec()
		} else {
			let max_len = self
				.query_results_type
				.iter()
				.map(|s| s.to_string().len())
				.max();

			self.query_results_type
				.iter()
				.enumerate()
				.take(height)
				.map(|(idx, commit_type)| {
					let text_string = format!(
						"{:w$} [{}]",
						commit_type,
						quick_shortcuts[idx],
						w = max_len.unwrap_or_default(),
					);
					let text = trim_length_left(&text_string, width);

					(
						self.selected_index == idx,
						format!("{text}{:width$}", " "),
					)
				})
				.collect_vec()
		};

		let items = iter_over.into_iter().map(|(selected, text)| {
			Line::from(
				text.graphemes(true)
					.enumerate()
					.map(|(_, c)| {
						Span::styled(
							Cow::from(c.to_string()),
							self.theme.text(selected, selected),
						)
					})
					.collect::<Vec<_>>(),
			)
		});

		ui::draw_list_block(
			f,
			area,
			Block::default()
				.title(Span::styled(title, self.theme.title(true)))
				.borders(Borders::TOP),
			items,
		);
	}

	pub fn quick_shortcuts(&self) -> Vec<char> {
		let mut available_chars =
			('a'..='z').into_iter().collect_vec();

		[
			self.key_config.keys.move_down,
			self.key_config.keys.move_up,
			self.key_config.keys.exit_popup,
			self.key_config.keys.breaking,
			self.key_config.keys.exit,
			self.key_config.keys.insert,
		]
		.into_iter()
		.for_each(|k| {
			if let KeyCode::Char(c) = k.code {
				if let Some(char_to_remove_index) =
					available_chars.iter().position(|&ch| ch == c)
				{
					available_chars.remove(char_to_remove_index);
				}
			}
		});

		self.query_results_type
			.iter()
			.map(|commit_type| commit_type.to_string())
			.map(|s| {
				if let Some(ch) = s.chars()
					.into_iter()
					.find(|c| available_chars.contains(&c)) {
                    available_chars.retain(|&c| c != ch);
                    ch
                } else {
                    *available_chars.first().expect("Should already have at least one letter available")
                }
			})
        .collect_vec()
	}

	pub fn move_selection(&mut self, direction: ScrollType) {
		let new_selection = match direction {
			ScrollType::Up => self.selected_index.saturating_sub(1),
			ScrollType::Down => self.selected_index.saturating_add(1),
			_ => self.selected_index,
		};

		let new_selection = new_selection.clamp(
			0,
			self.query_results_type.len().saturating_sub(1),
		);

		self.selected_index = new_selection;
	}

	pub fn any_work_pending(&self) -> bool {
		false
	}

	fn update_query(&mut self) {
		if self
			.query
			.as_ref()
			.is_none_or(|q| q != self.input.get_text())
		{
			self.set_query(self.input.get_text().to_string());
		}
	}

	fn set_query(&mut self, query: String) {
		let query = query.to_lowercase();
		self.query = Some(query.clone());

		if let Some(commit_type) = &self.seleted_commit_type {
			self.query_results_more_info = commit_type
				.more_info()
				.iter()
				.filter(|more_info_commit| {
					more_info_commit
						.strings()
						.2
						.to_lowercase()
						.contains(&query)
				})
				.cloned()
				.collect_vec();

			if self.selected_index
				>= self.query_results_more_info.len()
			{
				self.selected_index = self
					.query_results_more_info
					.len()
					.saturating_sub(1);
			}
		} else {
			self.query_results_type = self
				.options
				.iter()
				.filter(|option| {
					option.to_string().to_lowercase().contains(&query)
				})
				.cloned()
				.collect_vec();

			if self.selected_index >= self.query_results_type.len() {
				self.selected_index =
					self.query_results_type.len().saturating_sub(1);
			}
		}
	}

	fn validate_escape(&mut self, commit_type: CommitType) {
		let (emoji, short_msg, _) = self.query_results_more_info
			[self.selected_index]
			.strings();
		self.queue.push(crate::queue::InternalEvent::OpenCommit);
		self.queue.push(
			crate::queue::InternalEvent::AddCommitMessage(format!(
				"{emoji} {commit_type}{}{} {short_msg}",
				if self.is_breaking { "!" } else { "" },
				if short_msg.is_empty() { "" } else { ":" },
			)),
		);
		self.hide();
	}

	fn next_step(&mut self) {
		self.selected_index = 0;
		self.is_insert = false;
		self.query = None;
		self.input.clear();
		self.update_query();
	}
}

impl DrawableComponent for ConventionalCommitPopup {
	fn draw(&self, f: &mut Frame, area: Rect) -> Result<()> {
		if self.is_visible {
			const MAX_SIZE: (u16, u16) = (50, 25);

			let area = ui::centered_rect_absolute(
				MAX_SIZE.0, MAX_SIZE.1, area,
			);

			f.render_widget(Clear, area);
			f.render_widget(
				Block::default()
					.borders(Borders::all())
					.style(self.theme.title(true))
					.title(Span::styled(
						if self.seleted_commit_type.is_some() {
							"Emoji of commit"
						} else {
							"Type of commit"
						},
						self.theme.title(true),
					))
					.title(if self.is_breaking {
						Span::styled(
							"[BREAKING]",
							self.theme.title(true),
						)
					} else {
						"".into()
					})
					.title(if self.is_insert {
						Span::styled(
							"[INSERT]",
							self.theme.title(true),
						)
						.into_right_aligned_line()
					} else {
						"".into()
					}),
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
		if self.is_visible() || force_all {
			if self.is_insert {
				out.push(CommandInfo::new(
					strings::commands::exit_insert(&self.key_config),
					true,
					true,
				));
			} else {
				out.push(CommandInfo::new(
					strings::commands::insert(&self.key_config),
					true,
					true,
				));

				out.push(CommandInfo::new(
					strings::commands::close_fuzzy_finder(
						&self.key_config,
					),
					true,
					true,
				));
			}

			out.push(CommandInfo::new(
				strings::commands::scroll(&self.key_config),
				true,
				true,
			));

			out.push(CommandInfo::new(
				strings::commands::open_submodule(&self.key_config),
				true,
				true,
			));
		}

		visibility_blocking(self)
	}
	fn event(
		&mut self,
		event: &crossterm::event::Event,
	) -> Result<EventState> {
		if self.is_visible() {
			if let Event::Key(key) = event {
				if key_match(key, self.key_config.keys.exit_popup) {
					if self.is_insert {
						self.is_insert = false;
					} else {
						self.hide();
					}
				} else if key_match(key, self.key_config.keys.enter) {
					if let Some(commit_type) =
						self.seleted_commit_type.clone()
					{
						self.validate_escape(commit_type);
					} else {
						if let Some(&commit) = self
							.query_results_type
							.get(self.selected_index)
						{
							self.seleted_commit_type = Some(commit);
							self.next_step();

							if commit.more_info().len() == 1 {
								self.validate_escape(commit);
							}
						}
					}
				} else if key_match(
					key,
					self.key_config.keys.breaking,
				) {
					self.is_breaking = !self.is_breaking;
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
					if self.is_insert {
						if self.input.event(&event)?.is_consumed() {
							self.update_query();
						}
					} else if key_match(
						key,
						self.key_config.keys.insert,
					) {
						self.is_insert = true;
					} else {
						if let KeyCode::Char(c) = key.code {
							if let Some(idx) = self
								.quick_shortcuts()
								.into_iter()
								.position(|ch| ch == c)
							{
								self.seleted_commit_type = Some(
									self.query_results_type[idx]
										.clone(),
								);
								self.next_step();
							}
						}
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
		self.next_step();
		self.is_visible = false;
		self.seleted_commit_type = None;
		self.query_results_type = CommitType::iter().collect_vec();
		self.query_results_more_info = Vec::new();
	}

	fn show(&mut self) -> Result<()> {
		self.is_visible = true;
		self.input.show()?;
		self.input.set_text(String::new());
		Ok(())
	}
}
