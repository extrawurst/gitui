use super::{
	visibility_blocking, CommandBlocking, CommandInfo, Component,
	DrawableComponent, EventState, ScrollType, TextInputComponent,
};
use crate::{
	keys::{key_match, SharedKeyConfig},
	queue::{InternalEvent, Queue},
	string_utils::trim_length_left,
	strings,
	ui::{self, style::SharedTheme},
};
use anyhow::Result;
use crossterm::event::Event;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::{
	backend::Backend,
	layout::{Constraint, Direction, Layout, Margin, Rect},
	text::{Span, Spans},
	widgets::{Block, Borders, Clear},
	Frame,
};
use std::borrow::Cow;

pub struct BranchFindPopup {
	queue: Queue,
	visible: bool,
	find_text: TextInputComponent,
	query: Option<String>,
	theme: SharedTheme,
	branches: Vec<String>,
	selection: usize,
	selected_index: Option<usize>,
	branches_filtered: Vec<(usize, Vec<usize>)>,
	key_config: SharedKeyConfig,
}

impl BranchFindPopup {
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
			branches: Vec::new(),
			branches_filtered: Vec::new(),
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

		self.branches_filtered.clear();

		if let Some(q) = &self.query {
			let matcher =
				fuzzy_matcher::skim::SkimMatcherV2::default();

			let mut branches = self
				.branches
				.iter()
				.enumerate()
				.filter_map(|a| {
					matcher
						.fuzzy_indices(a.1, q)
						.map(|(score, indices)| (score, a.0, indices))
				})
				.collect::<Vec<(_, _, _)>>();

			branches.sort_by(|(score1, _, _), (score2, _, _)| {
				score2.cmp(score1)
			});

			self.branches_filtered.extend(
				branches.into_iter().map(|entry| (entry.1, entry.2)),
			);
		}

		self.selection = 0;
		self.refresh_selection();
	}

	fn refresh_selection(&mut self) {
		let selection =
			self.branches_filtered.get(self.selection).map(|a| a.0);

		if self.selected_index != selection {
			self.selected_index = selection;

			let idx = self.selected_index;
			self.queue.push(InternalEvent::BranchFinderChanged(idx));
		}
	}

	pub fn open(&mut self, branches: Vec<String>) -> Result<()> {
		self.show()?;
		self.find_text.show()?;
		self.find_text.set_text(String::new());
		self.query = None;
		if self.branches != branches {
			self.branches = branches;
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
			.clamp(0, self.branches_filtered.len().saturating_sub(1));

		if new_selection != self.selection {
			self.selection = new_selection;
			self.refresh_selection();
			return true;
		}

		false
	}
}

impl DrawableComponent for BranchFindPopup {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		area: Rect,
	) -> Result<()> {
		if self.is_visible() {
			const MAX_SIZE: (u16, u16) = (50, 20);

			let any_hits = !self.branches_filtered.is_empty();

			let area = ui::centered_rect_absolute(
				MAX_SIZE.0, MAX_SIZE.1, area,
			);

			let area = if any_hits {
				area
			} else {
				Layout::default()
					.direction(Direction::Vertical)
					.constraints(
						[
							Constraint::Length(3),
							Constraint::Percentage(100),
						]
						.as_ref(),
					)
					.split(area)[0]
			};

			f.render_widget(Clear, area);
			f.render_widget(
				Block::default()
					.borders(Borders::all())
					.style(self.theme.title(true))
					.title(Span::styled(
						strings::POPUP_TITLE_FUZZY_FIND,
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
				.split(area.inner(&Margin {
					horizontal: 1,
					vertical: 1,
				}));

			self.find_text.draw(f, chunks[0])?;

			if any_hits {
				let title =
					format!("Hits: {}", self.branches_filtered.len());

				let height = usize::from(chunks[1].height);
				let width = usize::from(chunks[1].width);

				let items = self
					.branches_filtered
					.iter()
					.take(height)
					.map(|(idx, indicies)| {
						let selected = self
							.selected_index
							.map_or(false, |index| index == *idx);
						let full_text = trim_length_left(
							&self.branches[*idx],
							width,
						);
						Spans::from(
							full_text
								.char_indices()
								.map(|(c_idx, c)| {
									Span::styled(
										Cow::from(c.to_string()),
										self.theme.text(
											selected,
											indicies.contains(&c_idx),
										),
									)
								})
								.collect::<Vec<_>>(),
						)
					});

				ui::draw_list_block(
					f,
					chunks[1],
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
		Ok(())
	}
}

impl Component for BranchFindPopup {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.is_visible() || force_all {
			out.push(CommandInfo::new(
				strings::commands::scroll_popup(&self.key_config),
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

		visibility_blocking(self)
	}

	fn event(
		&mut self,
		event: &crossterm::event::Event,
	) -> Result<EventState> {
		if self.is_visible() {
			if let Event::Key(key) = event {
				if key_match(key, self.key_config.keys.exit_popup)
					|| key_match(key, self.key_config.keys.enter)
				{
					self.hide();
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
