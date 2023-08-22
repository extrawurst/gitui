use super::{
	visibility_blocking, CommandBlocking, CommandInfo, Component,
	DrawableComponent, EventState, TextInputComponent,
};
use crate::{
	keys::{key_match, SharedKeyConfig},
	queue::{InternalEvent, Queue},
	strings::{self},
	ui::{self, style::SharedTheme},
};
use anyhow::Result;
use asyncgit::sync::{
	LogFilterSearchOptions, SearchFields, SearchOptions,
};
use crossterm::event::Event;
use ratatui::{
	backend::Backend,
	layout::{
		Alignment, Constraint, Direction, Layout, Margin, Rect,
	},
	text::{Line, Span},
	widgets::{Block, Borders, Clear, Paragraph},
	Frame,
};

enum Selection {
	EnterText,
	FuzzyOption,
	CaseOption,
	MessageSearch,
	FilenameSearch,
	AuthorsSearch,
}

pub struct LogSearchPopupComponent {
	queue: Queue,
	visible: bool,
	selection: Selection,
	key_config: SharedKeyConfig,
	find_text: TextInputComponent,
	options: (SearchFields, SearchOptions),
	theme: SharedTheme,
}

impl LogSearchPopupComponent {
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
			"search text",
			false,
		);
		find_text.embed();

		Self {
			queue: queue.clone(),
			visible: false,
			key_config,
			options: (
				SearchFields::default(),
				SearchOptions::default(),
			),
			theme,
			find_text,
			selection: Selection::EnterText,
		}
	}

	pub fn open(&mut self) -> Result<()> {
		self.show()?;
		self.find_text.show()?;
		self.find_text.set_text(String::new());

		Ok(())
	}

	fn execute_search(&mut self) {
		self.hide();

		if !self.find_text.get_text().trim().is_empty() {
			self.queue.push(InternalEvent::CommitSearch(
				LogFilterSearchOptions {
					fields: self.options.0,
					options: self.options.1,
					search_pattern: self
						.find_text
						.get_text()
						.to_string(),
				},
			));
		}
	}

	fn get_text_options(&self) -> Vec<Line> {
		let x_message =
			if self.options.0.contains(SearchFields::MESSAGE) {
				"X"
			} else {
				" "
			};

		let x_files =
			if self.options.0.contains(SearchFields::FILENAMES) {
				"X"
			} else {
				" "
			};

		let x_authors =
			if self.options.0.contains(SearchFields::AUTHORS) {
				"X"
			} else {
				" "
			};

		let x_opt_fuzzy =
			if self.options.1.contains(SearchOptions::FUZZY_SEARCH) {
				"X"
			} else {
				" "
			};

		let x_opt_casesensitive =
			if self.options.1.contains(SearchOptions::CASE_SENSITIVE)
			{
				"X"
			} else {
				" "
			};

		vec![
			Line::from(vec![Span::styled(
				format!("[{x_opt_fuzzy}] fuzzy search"),
				self.theme.text(
					matches!(self.selection, Selection::FuzzyOption),
					false,
				),
			)]),
			Line::from(vec![Span::styled(
				format!("[{x_opt_casesensitive}] case sensitive"),
				self.theme.text(
					matches!(self.selection, Selection::CaseOption),
					false,
				),
			)]),
			Line::from(vec![Span::styled(
				format!("[{x_message}] messages",),
				self.theme.text(
					matches!(
						self.selection,
						Selection::MessageSearch
					),
					false,
				),
			)]),
			Line::from(vec![Span::styled(
				format!("[{x_files}] commited files",),
				self.theme.text(
					matches!(
						self.selection,
						Selection::FilenameSearch
					),
					false,
				),
			)]),
			Line::from(vec![Span::styled(
				format!("[{x_authors}] authors",),
				self.theme.text(
					matches!(
						self.selection,
						Selection::AuthorsSearch
					),
					false,
				),
			)]),
			// Line::from(vec![Span::styled(
			// 	"[ ] changes (soon)",
			// 	theme,
			// )]),
			// Line::from(vec![Span::styled(
			// 	"[ ] hashes (soon)",
			// 	theme,
			// )]),
		]
	}

	fn option_selected(&self) -> bool {
		!matches!(self.selection, Selection::EnterText)
	}

	fn toggle_option(&mut self) {
		match self.selection {
			Selection::EnterText => (),
			Selection::FuzzyOption => {
				self.options.1.toggle(SearchOptions::FUZZY_SEARCH);
			}
			Selection::CaseOption => {
				self.options.1.toggle(SearchOptions::CASE_SENSITIVE);
			}
			Selection::MessageSearch => {
				self.options.0.toggle(SearchFields::MESSAGE);

				if self.options.0.is_empty() {
					self.options.0.set(SearchFields::FILENAMES, true);
				}
			}
			Selection::FilenameSearch => {
				self.options.0.toggle(SearchFields::FILENAMES);

				if self.options.0.is_empty() {
					self.options.0.set(SearchFields::AUTHORS, true);
				}
			}
			Selection::AuthorsSearch => {
				self.options.0.toggle(SearchFields::AUTHORS);

				if self.options.0.is_empty() {
					self.options.0.set(SearchFields::MESSAGE, true);
				}
			}
		}
	}

	fn move_selection(&mut self, arg: bool) {
		if arg {
			//up
			self.selection = match self.selection {
				Selection::EnterText => Selection::AuthorsSearch,
				Selection::FuzzyOption => Selection::EnterText,
				Selection::CaseOption => Selection::FuzzyOption,
				Selection::MessageSearch => Selection::CaseOption,
				Selection::FilenameSearch => Selection::MessageSearch,
				Selection::AuthorsSearch => Selection::FilenameSearch,
			};
		} else {
			self.selection = match self.selection {
				Selection::EnterText => Selection::FuzzyOption,
				Selection::FuzzyOption => Selection::CaseOption,
				Selection::CaseOption => Selection::MessageSearch,
				Selection::MessageSearch => Selection::FilenameSearch,
				Selection::FilenameSearch => Selection::AuthorsSearch,
				Selection::AuthorsSearch => Selection::EnterText,
			};
		}
	}
}

impl DrawableComponent for LogSearchPopupComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		area: Rect,
	) -> Result<()> {
		if self.is_visible() {
			const SIZE: (u16, u16) = (60, 10);
			let area =
				ui::centered_rect_absolute(SIZE.0, SIZE.1, area);

			f.render_widget(Clear, area);
			f.render_widget(
				Block::default()
					.borders(Borders::all())
					.style(self.theme.title(true))
					.title(Span::styled(
						strings::POPUP_TITLE_LOG_SEARCH,
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

			f.render_widget(
				Paragraph::new(self.get_text_options())
					.block(
						Block::default()
							.borders(Borders::TOP)
							.border_style(self.theme.block(true)),
					)
					.alignment(Alignment::Left),
				chunks[1],
			);
		}

		Ok(())
	}
}

impl Component for LogSearchPopupComponent {
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
					strings::commands::navigate_tree(
						&self.key_config,
					),
					true,
					true,
				)
				.order(1),
			);

			out.push(
				CommandInfo::new(
					strings::commands::toggle_option(
						&self.key_config,
					),
					self.option_selected(),
					true,
				)
				.order(1),
			);

			out.push(CommandInfo::new(
				strings::commands::confirm_action(&self.key_config),
				!self.find_text.get_text().trim().is_empty(),
				self.visible,
			));
		}

		visibility_blocking(self)
	}

	fn event(
		&mut self,
		event: &crossterm::event::Event,
	) -> Result<EventState> {
		if self.is_visible() {
			if let Event::Key(key) = &event {
				if key_match(key, self.key_config.keys.exit_popup) {
					self.hide();
				} else if key_match(key, self.key_config.keys.enter)
					&& !self.find_text.get_text().trim().is_empty()
				{
					self.execute_search();
				} else if key_match(key, self.key_config.keys.move_up)
				{
					self.move_selection(true);
				} else if key_match(
					key,
					self.key_config.keys.move_down,
				) {
					self.move_selection(false);
				} else if key_match(
					key,
					self.key_config.keys.log_mark_commit,
				) && self.option_selected()
				{
					self.toggle_option();
				}
			}

			if !self.option_selected()
				&& self.find_text.event(event)?.is_consumed()
			{
				return Ok(EventState::Consumed);
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
