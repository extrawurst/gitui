use crate::keys::key_match;
use crate::strings::symbol;
use crate::ui::Size;
use crate::{
	components::{
		popup_paragraph, visibility_blocking, CommandBlocking,
		CommandInfo, Component, DrawableComponent, EventState,
	},
	keys::SharedKeyConfig,
	strings,
	ui::{self, style::SharedTheme},
};
use anyhow::Result;
use core::cmp::{max, min};
use crossterm::event::{Event, KeyCode, KeyModifiers};
use itertools::Itertools;
use std::{cell::Cell, collections::HashMap, ops::Range};
use tui::{
	backend::Backend,
	layout::{Alignment, Rect},
	style::Modifier,
	text::{Spans, Text},
	widgets::{Clear, Paragraph},
	Frame,
};

#[derive(PartialEq, Eq)]
pub enum InputType {
	Singleline,
	Multiline,
	Password,
}

/// primarily a subcomponet for user input of text (used in `CommitComponent`)
pub struct TextInputComponent {
	title: String,
	default_msg: String,
	msg: String,
	visible: bool,
	show_char_count: bool,
	theme: SharedTheme,
	key_config: SharedKeyConfig,
	cursor_position: usize,
	input_type: InputType,
	current_area: Cell<Rect>,
	embed: bool,
	scroll_top: usize, // The current scroll from the top
	cur_line: usize,   // The current line
	scroll_max: usize, // The number of lines
	frame_height: Cell<usize>,
}

impl TextInputComponent {
	///
	pub fn new(
		theme: SharedTheme,
		key_config: SharedKeyConfig,
		title: &str,
		default_msg: &str,
		show_char_count: bool,
	) -> Self {
		Self {
			msg: String::new(),
			visible: false,
			theme,
			key_config,
			show_char_count,
			title: title.to_string(),
			default_msg: default_msg.to_string(),
			cursor_position: 0,
			input_type: InputType::Multiline,
			current_area: Cell::new(Rect::default()),
			embed: false,
			scroll_top: 0,
			cur_line: 0,
			scroll_max: 0,
			frame_height: Cell::new(0),
		}
	}

	pub const fn with_input_type(
		mut self,
		input_type: InputType,
	) -> Self {
		self.input_type = input_type;
		self
	}

	/// Clear the `msg`.
	pub fn clear(&mut self) {
		self.msg.clear();
		self.cursor_position = 0;
	}

	/// Get the `msg`.
	pub fn get_text(&self) -> &str {
		self.msg.as_str()
	}

	/// screen area (last time we got drawn)
	pub fn get_area(&self) -> Rect {
		self.current_area.get()
	}

	/// embed into parent draw area
	pub fn embed(&mut self) {
		self.embed = true;
	}

	/// Only for multiline
	fn insert_new_line(&mut self) {
		const BORDER_SIZE: usize = 1;

		// if the last line is just a new line with no
		// characters, and the cursor position is just before
		//it (on the previous line), shift to the new line
		if self.msg.ends_with('\n')
			&& self.cursor_position
				== self.msg.chars().count().saturating_sub(1)
		{
			self.incr_cursor();
			return;
		}

		self.msg.insert(self.cursor_position, '\n');
		self.incr_cursor();
		self.scroll_max += 1;

		// if the text box height increased,
		// compensate by scrolling up one
		if self.scroll_max
			< (self.frame_height.get())
				.saturating_sub(BORDER_SIZE * 2)
		{
			self.scroll_top = self.scroll_top.saturating_sub(1);
		}
	}

	/// See `incr_cursor`
	fn incr_cursor_multiline(&mut self) {
		if self.msg.chars().nth(self.cursor_position) == Some('\n') {
			self.cur_line += 1;
			if self.cur_line.saturating_sub(self.scroll_top)
				> (self.frame_height.get()).saturating_sub(3)
			{
				self.scroll_top += 1;
			}
		}
	}

	/// Move the cursor right one char.
	fn incr_cursor(&mut self) {
		if let Some(pos) = self.next_char_position() {
			if self.input_type == InputType::Multiline {
				self.incr_cursor_multiline();
			}
			self.cursor_position = pos;
		}
	}

	/// See `decr_cursor`
	fn decr_cursor_multiline(&mut self, index: usize) {
		if self.msg.chars().nth(index) == Some('\n') {
			self.cur_line = self.cur_line.saturating_sub(1);
			if self.cur_line < self.scroll_top {
				self.scroll_top = self.scroll_top.saturating_sub(1);
			}
		}
	}

	/// Move the cursor left one char.
	fn decr_cursor(&mut self) {
		let mut index = self.cursor_position.saturating_sub(1);
		while index > 0 && !self.msg.is_char_boundary(index) {
			index -= 1;
		}
		self.cursor_position = index;
		if self.input_type == InputType::Multiline {
			self.decr_cursor_multiline(index);
		}
	}

	/// Move the cursor up a line.
	/// Only for multi-line textinputs
	fn line_up_cursor(&mut self) {
		let mut top_line_start: usize = 0;
		let mut top_line_end: usize = 0;
		let mut middle_line_start: usize = 0;
		let mut middle_line_end: usize = 0;
		let mut bottom_line_start: usize = 0;

		for (i, c) in self.msg.chars().enumerate() {
			if c == '\n' {
				top_line_start = middle_line_start;
				top_line_end = middle_line_end;
				middle_line_start = bottom_line_start;
				middle_line_end = i.saturating_sub(1);
				bottom_line_start = i;
			}

			if i == self.cursor_position {
				//for when cursor position is on a new line just before text
				if c == '\n' {
					bottom_line_start = middle_line_start;
					middle_line_start = top_line_start;
					middle_line_end = top_line_end;
				}
				break;
			}
		}

		let mut cursor_position_in_line =
			self.cursor_position.saturating_sub(bottom_line_start);

		//for when moving up to first line
		if middle_line_start == 0 {
			cursor_position_in_line =
				cursor_position_in_line.saturating_sub(1);
		}
		self.cursor_position =
			middle_line_start.saturating_add(cursor_position_in_line);

		//for when moving uo to a new line from a line with characters
		if self.cursor_position > middle_line_end {
			self.cursor_position = middle_line_end.saturating_add(1);
		}

		while !self.msg.is_char_boundary(self.cursor_position) {
			self.cursor_position += 1;
		}
		self.cur_line = self.cur_line.saturating_sub(1);
		if self.cur_line < self.scroll_top {
			self.scroll_top = self.scroll_top.saturating_sub(1);
		}
	}

	/// Move the cursor down a line.
	/// Only for multi-line textinputs
	fn line_down_cursor(&mut self) {
		let mut top_line_start: usize = 0;
		let mut middle_line_start: usize = 0;
		let mut middle_line_end: usize = 0;
		let mut bottom_line_start: usize = 0;
		let mut drop_count: usize = 0;

		for (i, c) in self.msg.chars().enumerate() {
			if c == '\n' {
				top_line_start = middle_line_start;
				middle_line_start = bottom_line_start;
				middle_line_end = i.saturating_sub(1);
				bottom_line_start = i;

				if i >= self.cursor_position {
					drop_count += 1;
				}
			}

			if drop_count == 2 {
				break;
			}

			if i == self.msg.len().saturating_sub(1) && c != '\n' {
				top_line_start = middle_line_start;
				middle_line_start = bottom_line_start;
				middle_line_end = i.saturating_sub(1);
			} else if i == self.msg.len().saturating_sub(1)
				&& c == '\n'
			{
				top_line_start = middle_line_start;
				middle_line_start = bottom_line_start;
				middle_line_end = bottom_line_start;
			}
		}

		let mut cursor_position_in_line =
			self.cursor_position.saturating_sub(top_line_start);

		if top_line_start == 0 {
			cursor_position_in_line += 1;
		}
		self.cursor_position =
			middle_line_start.saturating_add(cursor_position_in_line);

		if self.cursor_position > middle_line_end
			&& self.cursor_position
				!= middle_line_end.saturating_add(1)
		{
			self.cursor_position = middle_line_end + 1;
		}

		if self.cursor_position < self.msg.len() {
			while !self.msg.is_char_boundary(self.cursor_position) {
				self.cursor_position += 1;
			}
		}

		if self.cur_line < self.scroll_max {
			self.cur_line += 1;
			if self.cur_line
				> self.scroll_top
					+ (self.current_area.get().height as usize)
						.saturating_sub(3_usize)
			{
				self.scroll_top += 1;
			}
		}
	}

	/// Get the position of the next char, or, if the cursor points
	/// to the last char, the `msg.len()`.
	/// Returns None when the cursor is already at `msg.len()`.
	fn next_char_position(&self) -> Option<usize> {
		if self.cursor_position >= self.msg.len() {
			return None;
		}
		let mut index = self.cursor_position.saturating_add(1);
		while index < self.msg.len()
			&& !self.msg.is_char_boundary(index)
		{
			index += 1;
		}
		Some(index)
	}

	/// Backspace for multiline textinputs
	fn multiline_backspace(&mut self) {
		const BORDER_SIZE: usize = 1;
		if self.msg.chars().nth(self.cursor_position) == Some('\n') {
			self.scroll_max -= 1;
			if !(self.scroll_max
				< (self.frame_height.get() as usize)
					.saturating_sub(BORDER_SIZE * 2)
				&& self.scroll_max >= 3)
			{
				self.scroll_top = self.scroll_top.saturating_sub(1);
			}
		}
	}

	fn backspace(&mut self) {
		if self.cursor_position > 0 {
			self.decr_cursor();
			if self.input_type == InputType::Multiline {
				self.multiline_backspace();
			}
			self.msg.remove(self.cursor_position);
		}
	}

	/// Set the `msg`.
	pub fn set_text(&mut self, msg: String) {
		self.msg = msg;
		self.cursor_position = 0;
	}

	/// Set the `title`.
	pub fn set_title(&mut self, t: String) {
		self.title = t;
	}

	///
	pub fn set_default_msg(&mut self, v: String) {
		self.default_msg = v;
	}

	#[allow(unstable_name_collisions)]
	fn get_draw_text(&self) -> Text {
		let style = self.theme.text(true, false);

		let mut txt = Text::default();
		// The portion of the text before the cursor is added
		// if the cursor is not at the first character.
		if self.cursor_position > 0 {
			let text_before_cursor: String = self
				.get_msg(0..self.cursor_position)
				.split('\n')
				.skip(self.scroll_top)
				.intersperse("\n")
				.collect();
			let ends_in_nl = text_before_cursor.ends_with('\n');
			txt = text_append(
				txt,
				Text::styled(text_before_cursor, style),
			);
			if ends_in_nl {
				txt.lines.push(Spans::default());
			}
		}

		let cursor_str = self
			.next_char_position()
			// if the cursor is at the end of the msg
			// a whitespace is used to underline
			.map_or(" ".to_owned(), |pos| {
				self.get_msg(self.cursor_position..pos)
			});

		let cursor_highlighting = {
			let mut h = HashMap::with_capacity(2);
			h.insert("\n", "\u{21b5}\n\r");
			h.insert(" ", symbol::WHITESPACE);
			h
		};

		if let Some(substitute) =
			cursor_highlighting.get(cursor_str.as_str())
		{
			txt = text_append(
				txt,
				Text::styled(
					substitute.to_owned(),
					self.theme
						.text(false, false)
						.add_modifier(Modifier::UNDERLINED),
				),
			);
		} else {
			txt = text_append(
				txt,
				Text::styled(
					cursor_str,
					style.add_modifier(Modifier::UNDERLINED),
				),
			);
		}

		// The final portion of the text is added if there are
		// still remaining characters.
		if let Some(pos) = self.next_char_position() {
			if pos < self.msg.len() {
				txt = text_append(
					txt,
					Text::styled(
						self.get_msg(pos..self.msg.len()),
						style,
					),
				);
			}
		}

		txt
	}

	fn get_msg(&self, range: Range<usize>) -> String {
		match self.input_type {
			InputType::Password => range.map(|_| "*").join(""),
			_ => self.msg[range].to_owned(),
		}
	}

	fn draw_char_count<B: Backend>(&self, f: &mut Frame<B>, r: Rect) {
		let count = self.msg.len();
		if count > 0 {
			let w = Paragraph::new(format!("[{} chars]", count))
				.alignment(Alignment::Right);

			let mut rect = {
				let mut rect = r;
				rect.y += rect.height.saturating_sub(1);
				rect
			};

			rect.x += 1;
			rect.width = rect.width.saturating_sub(2);
			rect.height = rect
				.height
				.saturating_sub(rect.height.saturating_sub(1));

			f.render_widget(w, rect);
		}
	}
}

// merges last line of `txt` with first of `append` so we do not generate unneeded newlines
fn text_append<'a>(txt: Text<'a>, append: Text<'a>) -> Text<'a> {
	let mut txt = txt;
	if let Some(last_line) = txt.lines.last_mut() {
		if let Some(first_line) = append.lines.first() {
			last_line.0.extend(first_line.0.clone());
		}

		if append.lines.len() > 1 {
			for line in 1..append.lines.len() {
				let spans = append.lines[line].clone();
				txt.lines.push(spans);
			}
		}
	} else {
		txt = append;
	}
	txt
}

impl DrawableComponent for TextInputComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		rect: Rect,
	) -> Result<()> {
		if self.visible {
			let txt = if self.msg.is_empty() {
				Text::styled(
					self.default_msg.as_str(),
					self.theme.text(false, false),
				)
			} else {
				self.get_draw_text()
			};

			let area = match self.input_type {
				InputType::Multiline => {
					let area = ui::centered_rect(60, 20, f.size());
					ui::rect_inside(
						Size::new(
							10,
							min(
								max(
									3,
									self.msg
										.chars()
										.filter(|x| *x == '\n')
										.count()
										.saturating_add(3)
										.try_into()
										.expect("Cannot fail"),
								),
								f.size().height,
							),
						),
						f.size().into(),
						area,
					)
				}
				_ => ui::centered_rect_absolute(32, 3, f.size()),
			};

			f.render_widget(Clear, area);
			f.render_widget(
				popup_paragraph(
					self.title.as_str(),
					txt,
					&self.theme,
					true,
					!self.embed,
				),
				area,
			);

			if self.show_char_count {
				self.draw_char_count(f, area);
			}

			if self.input_type == InputType::Multiline
				&& self.scroll_max > self.frame_height.get()
			{
				ui::draw_scrollbar(
					f,
					area,
					&self.theme,
					self.scroll_max,
					self.cur_line,
				);
			}

			self.current_area.set(area);
			self.frame_height.set(f.size().height as usize);
		}

		Ok(())
	}
}

impl Component for TextInputComponent {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		_force_all: bool,
	) -> CommandBlocking {
		out.push(
			CommandInfo::new(
				strings::commands::close_popup(&self.key_config),
				true,
				self.visible,
			)
			.order(1),
		);

		if self.input_type == InputType::Multiline {
			out.push(CommandInfo::new(
				strings::commands::commit_new_line(&self.key_config),
				true,
				self.visible,
			));
		}

		visibility_blocking(self)
	}

	fn event(&mut self, ev: &Event) -> Result<EventState> {
		if self.visible {
			if let Event::Key(e) = ev {
				if key_match(e, self.key_config.keys.exit_popup) {
					self.hide();
					return Ok(EventState::Consumed);
				} else if key_match(e, self.key_config.keys.enter)
					&& self.input_type == InputType::Multiline
				{
					self.insert_new_line();
					return Ok(EventState::Consumed);
				}

				let is_ctrl =
					e.modifiers.contains(KeyModifiers::CONTROL);

				match e.code {
					KeyCode::Char(c) if !is_ctrl => {
						self.msg.insert(self.cursor_position, c);
						self.incr_cursor();
						return Ok(EventState::Consumed);
					}
					KeyCode::Delete => {
						if self.cursor_position < self.msg.len() {
							self.msg.remove(self.cursor_position);
						}
						return Ok(EventState::Consumed);
					}
					KeyCode::Backspace => {
						self.backspace();
						return Ok(EventState::Consumed);
					}
					KeyCode::Left => {
						self.decr_cursor();
						return Ok(EventState::Consumed);
					}
					KeyCode::Right => {
						self.incr_cursor();
						return Ok(EventState::Consumed);
					}
					KeyCode::Up
						if self.input_type
							== InputType::Multiline =>
					{
						self.line_up_cursor();
						return Ok(EventState::Consumed);
					}
					KeyCode::Down
						if self.input_type
							== InputType::Multiline =>
					{
						self.line_down_cursor();
						return Ok(EventState::Consumed);
					}
					KeyCode::Home => {
						self.cursor_position = 0;
						return Ok(EventState::Consumed);
					}
					KeyCode::End => {
						self.cursor_position = self.msg.len();
						return Ok(EventState::Consumed);
					}
					_ => (),
				};
			}
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

#[cfg(test)]
mod tests {
	use super::*;
	use tui::{style::Style, text::Span};

	#[test]
	fn test_smoke() {
		let mut comp = TextInputComponent::new(
			SharedTheme::default(),
			SharedKeyConfig::default(),
			"",
			"",
			false,
		);

		comp.set_text(String::from("a\nb"));

		assert_eq!(comp.cursor_position, 0);

		comp.incr_cursor();
		assert_eq!(comp.cursor_position, 1);

		comp.decr_cursor();
		assert_eq!(comp.cursor_position, 0);
	}

	#[test]
	fn text_cursor_initial_position() {
		let mut comp = TextInputComponent::new(
			SharedTheme::default(),
			SharedKeyConfig::default(),
			"",
			"",
			false,
		);
		let theme = SharedTheme::default();
		let underlined = theme
			.text(true, false)
			.add_modifier(Modifier::UNDERLINED);

		comp.set_text(String::from("a"));

		let txt = comp.get_draw_text();

		assert_eq!(txt.lines[0].0.len(), 1);
		assert_eq!(get_text(&txt.lines[0].0[0]), Some("a"));
		assert_eq!(get_style(&txt.lines[0].0[0]), Some(&underlined));
	}

	#[test]
	fn test_cursor_second_position() {
		let mut comp = TextInputComponent::new(
			SharedTheme::default(),
			SharedKeyConfig::default(),
			"",
			"",
			false,
		);
		let theme = SharedTheme::default();
		let underlined_whitespace = theme
			.text(false, false)
			.add_modifier(Modifier::UNDERLINED);

		let not_underlined = Style::default();

		comp.set_text(String::from("a"));
		comp.incr_cursor();

		let txt = comp.get_draw_text();

		assert_eq!(txt.lines[0].0.len(), 2);
		assert_eq!(get_text(&txt.lines[0].0[0]), Some("a"));
		assert_eq!(
			get_style(&txt.lines[0].0[0]),
			Some(&not_underlined)
		);
		assert_eq!(
			get_text(&txt.lines[0].0[1]),
			Some(symbol::WHITESPACE)
		);
		assert_eq!(
			get_style(&txt.lines[0].0[1]),
			Some(&underlined_whitespace)
		);
	}

	#[test]
	fn test_visualize_newline() {
		let mut comp = TextInputComponent::new(
			SharedTheme::default(),
			SharedKeyConfig::default(),
			"",
			"",
			false,
		);

		let theme = SharedTheme::default();
		let underlined = theme
			.text(false, false)
			.add_modifier(Modifier::UNDERLINED);

		comp.set_text(String::from("a\nb"));
		comp.incr_cursor();

		let txt = comp.get_draw_text();

		assert_eq!(txt.lines.len(), 2);
		assert_eq!(txt.lines[0].0.len(), 2);
		assert_eq!(txt.lines[1].0.len(), 2);
		assert_eq!(get_text(&txt.lines[0].0[0]), Some("a"));
		assert_eq!(get_text(&txt.lines[0].0[1]), Some("\u{21b5}"));
		assert_eq!(get_style(&txt.lines[0].0[1]), Some(&underlined));
		assert_eq!(get_text(&txt.lines[1].0[0]), Some(""));
		assert_eq!(get_text(&txt.lines[1].0[1]), Some("b"));
	}

	#[test]
	fn test_invisible_newline() {
		let mut comp = TextInputComponent::new(
			SharedTheme::default(),
			SharedKeyConfig::default(),
			"",
			"",
			false,
		);

		let theme = SharedTheme::default();
		let underlined = theme
			.text(true, false)
			.add_modifier(Modifier::UNDERLINED);

		comp.set_text(String::from("a\nb"));

		let txt = comp.get_draw_text();

		assert_eq!(txt.lines.len(), 2);
		assert_eq!(txt.lines[0].0.len(), 2);
		assert_eq!(txt.lines[1].0.len(), 1);
		assert_eq!(get_text(&txt.lines[0].0[0]), Some("a"));
		assert_eq!(get_text(&txt.lines[0].0[1]), Some(""));
		assert_eq!(get_style(&txt.lines[0].0[0]), Some(&underlined));
		assert_eq!(get_text(&txt.lines[1].0[0]), Some("b"));
	}

	fn get_text<'a>(t: &'a Span) -> Option<&'a str> {
		Some(&t.content)
	}

	fn get_style<'a>(t: &'a Span) -> Option<&'a Style> {
		Some(&t.style)
	}
}
