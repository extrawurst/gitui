use super::{
	visibility_blocking, CommandBlocking, CommandInfo, Component,
	DrawableComponent, EventState, ScrollType, TextInputComponent,
};
use crate::{
	keys::SharedKeyConfig,
	queue::{InternalEvent, Queue},
	string_utils::trim_length_left,
	strings,
	ui::{self, style::SharedTheme},
};
use anyhow::Result;
use asyncgit::sync::TreeFile;
use crossterm::event::Event;
use fuzzy_matcher::FuzzyMatcher;
use std::borrow::Cow;
use tui::{
	backend::Backend,
	layout::{Constraint, Direction, Layout, Margin, Rect},
	text::Span,
	widgets::{Block, Borders, Clear},
	Frame,
};

pub struct FileFindPopup {
	queue: Queue,
	visible: bool,
	find_text: TextInputComponent,
	query: Option<String>,
	theme: SharedTheme,
	files: Vec<TreeFile>,
	selection: usize,
	selected_index: Option<usize>,
	files_filtered: Vec<usize>,
	key_config: SharedKeyConfig,
}

impl FileFindPopup {
	///
	pub fn new(
		queue: &Queue,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		let mut find_text = TextInputComponent::new(
			theme.clone(),
			key_config.clone(),
			"",
			"start typing..",
			false,
		);
		find_text.embed();

		Self {
			queue: queue.clone(),
			visible: false,
			query: None,
			find_text,
			theme,
			files: Vec::new(),
			files_filtered: Vec::new(),
			selected_index: None,
			key_config,
			selection: 0,
		}
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
						//TODO: use fuzzy_indices and highlight hits
						matcher.fuzzy_match(path, q).map(|_| a.0)
					})
				}),
			);

			self.selection = 0;
			self.refresh_selection();
		} else {
			self.files_filtered
				.extend(self.files.iter().enumerate().map(|a| a.0));
		}
	}

	fn refresh_selection(&mut self) {
		let selection =
			self.files_filtered.get(self.selection).copied();

		if self.selected_index != selection {
			self.selected_index = selection;

			let file = self
				.selected_index
				.and_then(|index| self.files.get(index))
				.map(|f| f.path.clone());

			self.queue.push(InternalEvent::FileFinderChanged(file));
		}
	}

	pub fn open(&mut self, files: &[TreeFile]) -> Result<()> {
		self.show()?;
		self.find_text.show()?;
		self.find_text.set_text(String::new());
		self.query = None;
		if self.files != *files {
			self.files = files.to_owned();
		}
		self.update_query();

		Ok(())
	}

	fn move_selection(&mut self, move_type: ScrollType) -> bool {
		let new_selection = match move_type {
			ScrollType::Up => self.selection.saturating_sub(1),
			ScrollType::Down => self.selection.saturating_add(1),
			_ => self.selection,
		};

		let new_selection = new_selection
			.clamp(0, self.files_filtered.len().saturating_sub(1));

		if new_selection != self.selection {
			self.selection = new_selection;
			self.refresh_selection();
			return true;
		}

		false
	}
}

impl DrawableComponent for FileFindPopup {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		area: Rect,
	) -> Result<()> {
		if self.is_visible() {
			const SIZE: (u16, u16) = (50, 25);
			let area =
				ui::centered_rect_absolute(SIZE.0, SIZE.1, area);

			f.render_widget(Clear, area);
			f.render_widget(
				Block::default()
					.borders(Borders::all())
					.style(self.theme.title(true))
					.title(Span::styled(
						//TODO: strings
						"Fuzzy find",
						self.theme.title(true),
					)),
				area,
			);

			let area = Layout::default()
				.direction(Direction::Vertical)
				.constraints(
					[
						Constraint::Length(1),
						Constraint::Percentage(100),
					]
					.as_ref(),
				)
				.split(area.inner(&Margin {
					horizontal: 1,
					vertical: 1,
				}));

			self.find_text.draw(f, area[0])?;

			let height = usize::from(area[1].height);
			let width = usize::from(area[1].width);

			let items =
				self.files_filtered.iter().take(height).map(|idx| {
					let selected = self
						.selected_index
						.map_or(false, |index| index == *idx);
					Span::styled(
						Cow::from(trim_length_left(
							self.files[*idx]
								.path
								.to_str()
								.unwrap_or_default(),
							width,
						)),
						self.theme.text(selected, false),
					)
				});

			let title =
				format!("Hits: {}", self.files_filtered.len());

			ui::draw_list_block(
				f,
				area[1],
				Block::default()
					.title(Span::styled(
						title,
						self.theme.title(true),
					))
					.borders(Borders::TOP),
				items,
			);
		}
		Ok(())
	}
}

impl Component for FileFindPopup {
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

			out.push(CommandInfo::new(
				strings::commands::scroll(&self.key_config),
				true,
				true,
			));
		}

		visibility_blocking(self)
	}

	fn event(
		&mut self,
		event: crossterm::event::Event,
	) -> Result<EventState> {
		if self.is_visible() {
			if let Event::Key(key) = &event {
				if *key == self.key_config.exit_popup
					|| *key == self.key_config.enter
				{
					self.hide();
				} else if *key == self.key_config.move_down {
					self.move_selection(ScrollType::Down);
				} else if *key == self.key_config.move_up {
					self.move_selection(ScrollType::Up);
				}
			}

			if self.find_text.event(event)?.is_consumed() {
				self.update_query();
			}

			return Ok(EventState::Consumed);
		}

		Ok(EventState::NotConsumed)
	}

	fn is_visible(&self) -> bool {
		self.visible
	}

	fn hide(&mut self) {
		self.visible = false;
	}

	fn show(&mut self) -> Result<()> {
		self.visible = true;
		Ok(())
	}
}
