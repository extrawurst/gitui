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
enum CommitType {}

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
			theme: env.theme.clone(),
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

			let items = self
				.query_results
				.iter()
				.enumerate()
				.take(height)
				.map(|(idx, commit_type)| {
					let selected = self.selected_index == idx;
					let commit_type_string = commit_type.to_string();
					let text = trim_length_left(
						commit_type_string.as_str(),
						width - 4, // ` [k]`
					);
					let text = format!(
						"{:w$} [{}]",
						text,
						quick_shortcuts[idx],
						w = width,
					);

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

		// println!("{} {}", self.query, self.input);

		let new_selection = new_selection.clamp(0, todo!());
		// .clamp(0, self.filtered.len().saturating_sub(1));
		// .clamp(0, self.filtered.len().saturating_sub(1));

		// if new_selection != self.selection {
		self.selected_index = new_selection;
		// 	return true;
		// }
		//
		// false
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

		// todo!()
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
					self.hide();
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

			// if self.find_text.event(event)?.is_consumed() {
			// 	self.update_query();
			// }

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
