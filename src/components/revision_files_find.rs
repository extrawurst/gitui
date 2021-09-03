#![allow(dead_code)]

use std::borrow::Cow;

use super::{
	CommandBlocking, CommandInfo, Component, DrawableComponent,
	EventState, TextInputComponent,
};
use crate::{
	keys::SharedKeyConfig,
	ui::{self, style::SharedTheme},
};
use anyhow::Result;
use asyncgit::sync::TreeFile;
use crossterm::event::Event;
use fuzzy_matcher::FuzzyMatcher;
use tui::{
	backend::Backend,
	layout::{Constraint, Direction, Layout, Rect},
	text::Span,
	widgets::{Block, Borders, Clear},
	Frame,
};

pub struct FileFindComponent {
	visible: bool,
	find_text: TextInputComponent,
	query: Option<String>,
	theme: SharedTheme,
	files: Vec<TreeFile>,
	files_filtered: Vec<usize>,
	key_config: SharedKeyConfig,
}

impl FileFindComponent {
	///
	pub fn new(
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		let mut find_text = TextInputComponent::new(
			theme.clone(),
			key_config.clone(),
			"find",
			"",
			false,
		);
		find_text.embed();

		Self {
			visible: false,
			query: None,
			find_text,
			theme,
			files: Vec::new(),
			files_filtered: Vec::new(),
			key_config,
		}
	}

	pub const fn is_visible(&self) -> bool {
		self.visible
	}

	pub fn hide(&mut self) {
		self.visible = false;
	}

	pub fn clear(&mut self) {
		self.files.clear();
	}

	pub fn open(&mut self, files: &[TreeFile]) -> Result<()> {
		self.visible = true;
		self.find_text.show()?;
		if self.files != *files {
			self.files = files.to_owned();
		}

		Ok(())
	}

	pub fn get_selection(&self) -> Option<&TreeFile> {
		self.files_filtered
			.first()
			.and_then(|idx| self.files.get(*idx))
	}

	fn update_query(&mut self) {
		if self.find_text.get_text().is_empty() {
			self.set_query(None);
		} else if self
			.query
			.as_ref()
			.map_or(true, |q| q != self.find_text.get_text())
		{
			self.set_query(Some(
				self.find_text.get_text().to_string(),
			));
		}
	}

	fn set_query(&mut self, query: Option<String>) {
		self.query = query;

		self.files_filtered.clear();

		if let Some(q) = &self.query {
			let matcher =
				fuzzy_matcher::skim::SkimMatcherV2::default();

			self.files_filtered.extend(
				self.files.iter().enumerate().filter_map(|a| {
					a.1.path.to_str().and_then(|path| {
						matcher.fuzzy_match(path, q).map(|_| a.0)
					})
				}),
			);
		} else {
			self.files_filtered
				.extend(self.files.iter().enumerate().map(|a| a.0));
		}
	}
}

impl DrawableComponent for FileFindComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		area: Rect,
	) -> Result<()> {
		if self.is_visible() {
			const SIZE: (u16, u16) = (50, 20);
			let area =
				ui::centered_rect_absolute(SIZE.0, SIZE.1, area);

			let chunks = Layout::default()
				.direction(Direction::Vertical)
				.constraints(
					[
						Constraint::Length(3),
						Constraint::Percentage(100),
					]
					.as_ref(),
				)
				.split(area);

			f.render_widget(Clear, area);

			self.find_text.draw(f, chunks[0])?;

			let items = self
				.files_filtered
				//TODO: scroll
				// .iterate(self.scroll.get_top(), tree_height)
				.iter()
				.map(|idx| {
					Span::raw(Cow::from(
						self.files[*idx]
							.path
							.to_str()
							.unwrap_or_default(),
					))
				});

			let title =
				format!("Hits: {}", self.files_filtered.len());

			ui::draw_list_block(
				f,
				chunks[1],
				Block::default()
					.title(Span::styled(
						title,
						self.theme.title(true),
					))
					.borders(Borders::ALL)
					.border_style(self.theme.block(true)),
				items,
			);
		}
		Ok(())
	}
}

impl Component for FileFindComponent {
	fn commands(
		&self,
		_out: &mut Vec<CommandInfo>,
		_force_all: bool,
	) -> CommandBlocking {
		CommandBlocking::PassingOn
	}

	fn event(
		&mut self,
		event: crossterm::event::Event,
	) -> Result<EventState> {
		if self.is_visible() {
			if let Event::Key(key) = &event {
				if *key == self.key_config.exit_popup {
					self.hide();
					return Ok(EventState::Consumed);
				}
			}

			if self.find_text.event(event)?.is_consumed() {
				self.update_query();
				return Ok(EventState::Consumed);
			}
		}

		Ok(EventState::NotConsumed)
	}
}
