#![allow(unused_imports)]
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
use crossterm::event::{Event, KeyCode, KeyModifiers};
use itertools::Itertools;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders};
use ratatui::{
	backend::Backend,
	layout::{Alignment, Rect},
	style::Modifier,
	text::Text,
	widgets::{Clear, Paragraph},
	Frame,
};
use std::cell::RefCell;
use std::convert::From;
use std::{cell::Cell, collections::HashMap, ops::Range};
use std::{path::PathBuf, rc::Rc};
use tui_textarea::{CursorMove, Input, Key, Scrolling, TextArea};
#[derive(PartialEq, Eq)]
pub enum InputType {
	Singleline,
	Multiline,
	Password,
}
/*
	completely rewritten using tui-textarea as it provides a ton of useful features
	- multiline edit
	- scroll vertically and horizontally
	- tab expansion
	- configurable masking
	- copy paste
	- ...

Notes to extrawurst

I tried to confine the changes to just this file as much as possible. Things that leaked out: -

==== ticks ======
TTA manages its text via &str rather than String. IE it expects the caller to own the text,
this introduces lifetimes into the TTA signature.

so all users of it now have
	 find_text: TextInputComponent<'a>,
rather then
	 find_text: TextInputComponent,

this in turn causes that struct to be ticked too, I call this a tick infestation. There is nothing to be done about it
(I tried real hard, unless you know a way).

	pub struct LogSearchPopupComponent<'a> {
	...
	impl<'a> DrawableComponent for LogSearchPopupComponent<'a>

	etc.

personally I think rustc should elide this, but it does not.

=== multiline vs singleline ===

since TTA support full multiline editing the choice of single line vs multiline matters
setting Singleline mode suppresses new line creation.

So I have set singleline mode on all callers that need it


=== crate upgrades ====

main one is crossterm bump to 0.27
this in turn pulls in other things , like bitflags v2 which messes with .ron files

=== key input ===

first. All keys work as before. You said 'dont break any key meanings'

I really would have liked to change the way components do key handling when using textinput. At present they all do
a variant of this
			// see if textinput wants it
			if self.input.event(ev)?.is_consumed() {
				return Ok(EventState::Consumed);
			}
			// no -see if it has special meaning to this component
			if let Event::Key(e) = ev {
				if key_match(e, self.key_config.keys.enter)
					&& self.is_valid_tag()
				{

ie pass the key to textinput then if it doesnt eat the key see if has special meaning for the component.

The problem with that is the textinput does not know the calling context so it doesnt know what keys have special meaning at the moment.
For example TTA uses ^f to mean word forward. But in the commit UI it means 'force'. However in other UIs it has no meaning. textinput
does not know whether or not it should accept ^f. I would have reversed the order: ie done

			// do we want the key?
			if let Event::Key(e) = ev {
				if key_match(e, self.key_config.keys.enter)
					&& self.is_valid_tag()
				{
					....
			// if not consumed here as 'special' pass to textinput
			if self.input.event(ev)?.is_consumed() {
				return Ok(EventState::Consumed);
			}

but that would have been a lot of change. So instead textinput ignores all keys that might be special for any UI. So , for example,
^f never works now. (But ctrl-left works)

New line is shift-enter, ctrl-enter or ^m. These are all common new line editor commands (discord, emacs,...)

There is no key mapping done in the TTA interface code. It looks at the key config to know which keys to ignore.
So for example if I remap 'commit force' to ^d (just do it), textinput will ignore ^d (and ^f will start working again).
But the key codes used by the editor itself are not read from the key config. I can certainly do that, I would add new set
of key names 'editor_up','editor_down' etc. Cannot use the current names as then are only used for navigation not input,
for example vim_keys uses 'k' to mean 'up'. Thats still useful but it would not make any sense to use 'k' as 'up' in textinput

Do you want key mapping? I can add later (since its a feature that doesnt exist in gitui today)

== help ==

There is no help for the editor window. The only thing a user really needs to know is the newline key stroke,
but there are 10-15 ctrl key codes. I could add then to the general help popup, or make a special one for textinput

here is complete list FYI

the ones wrapped in () are the ones ignored as 'special'

Ctrl+H, Backspace	Delete one character before cursor
Ctrl+D, Delete	Delete one character next to cursor
Ctrl+M, shif+Enter, ctrl+enter	Insert newline
Ctrl+K	Delete from cursor until the end of line
Ctrl+J	Delete from cursor until the head of line
Ctrl+W, Alt+H, Alt+Backspace	Delete one word before cursor
Alt+D, Alt+Delete	Delete one word next to cursor
Ctrl+U	Undo
Ctrl+R	Redo
Ctrl+Y	Paste yanked text
(Ctrl+F), →	Move cursor forward by one character
Ctrl+B, ←	Move cursor backward by one character
Ctrl+P, ↑	Move cursor up by one line
(Ctrl+N), ↓	Move cursor down by one line
Alt+F, Ctrl+→	Move cursor forward by word
Atl+B, Ctrl+←	Move cursor backward by word
Alt+], Alt+P, Ctrl+↑	Move cursor up by paragraph
Alt+[, Alt+N, Ctrl+↓	Move cursor down by paragraph
(Ctrl+E), End, Ctrl+Alt+F, Ctrl+Alt+→	Move cursor to the end of line
(Ctrl+A), Home, Ctrl+Alt+B, Ctrl+Alt+←	Move cursor to the head of line
Alt+<, Ctrl+Alt+P, Ctrl+Alt+↑	Move cursor to top of lines
Alt+>, Ctrl+Alt+N, Ctrl+Alt+↓	Move cursor to bottom of lines
Ctrl+V, PageDown	Scroll down by page
Alt+V, PageUp	Scroll up by page

== minor changes ==

get_text now returns a String rather that a &str. So in a few place I had to change to add 'as_str' to the call.

removed the tests for the multiline handling here because its now completely different
and TTA has tests for its own multi line handling

the word left and right test has been changed becuase the emoji handling in the
gitui code did not match what any other editors did with emojis (or chinese characters)
see explanation in the tests below

*/

/// primarily a subcomponet for user input of text (used in `CommitComponent`)

pub struct TextInputComponent<'a> {
	title: String,
	default_msg: String,
	selected: Option<bool>,
	msg: String,
	show_char_count: bool,
	theme: SharedTheme,
	key_config: SharedKeyConfig,
	input_type: InputType,
	current_area: Cell<Rect>,
	embed: bool,
	textarea: Option<TextArea<'a>>,
}

impl<'a> TextInputComponent<'a> {
	///
	pub fn new(
		theme: SharedTheme,
		key_config: SharedKeyConfig,
		title: &str,
		default_msg: &str,
		show_char_count: bool,
	) -> Self {
		Self {
			msg: String::from(""),
			theme,
			key_config,
			show_char_count,
			title: title.to_string(),
			default_msg: default_msg.to_string(),
			selected: None,
			input_type: InputType::Multiline,
			current_area: Cell::new(Rect::default()),
			embed: false,
			textarea: None,
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
		self.msg = String::from("");
		if self.is_visible() {
			self.create_and_show();
		}
	}

	/// Get the `msg`.
	pub fn get_text(&self) -> String {
		let text = if let Some(ta) = &self.textarea {
			ta.lines().join("\n")
		} else {
			String::from("")
		};
		text
	}

	/// screen area (last time we got drawn)
	pub fn get_area(&self) -> Rect {
		self.current_area.get()
	}

	/// embed into parent draw area
	pub fn embed(&mut self) {
		self.embed = true;
	}

	///
	pub fn enabled(&mut self, enable: bool) {
		self.selected = Some(enable);
	}

	//	create the textarea and then load it with the text
	//	from self.msg
	//  the recreate is done because some things can only be loaded at
	//  creation time.

	fn create_and_show(&mut self) {
		let lines: Vec<String> =
			self.msg.split("\n").map(|s| s.to_string()).collect();
		self.textarea = Some({
			let style =
				self.theme.text(self.selected.unwrap_or(true), false);
			let mut text_area = TextArea::new(lines);
			if self.input_type == InputType::Password {
				text_area.set_mask_char('*');
			}
			text_area
				.set_cursor_line_style(self.theme.text(true, false));
			text_area.set_placeholder_text(self.default_msg.clone());
			text_area.set_placeholder_style(style);
			text_area.set_style(style);
			if !self.embed {
				text_area.set_block(
					Block::default()
						.borders(Borders::ALL)
						.border_style(
							ratatui::style::Style::default()
								.add_modifier(
									ratatui::style::Modifier::BOLD,
								),
						)
						.title(self.title.clone()),
				);
			};
			text_area
		});
	}

	/// Set the `msg`.
	pub fn set_text(&mut self, msg: String) {
		self.msg = msg;
		if self.is_visible() {
			self.create_and_show();
		}
	}

	/// Set the `title`.
	pub fn set_title(&mut self, t: String) {
		self.title = t;
	}

	///
	pub fn set_default_msg(&mut self, v: String) {
		self.default_msg = v;
		if self.is_visible() {
			self.create_and_show();
		}
	}

	fn draw_char_count<B: Backend>(&self, f: &mut Frame<B>, r: Rect) {
		let count = self.get_text().len();
		if count > 0 {
			let w = Paragraph::new(format!("[{count} chars]"))
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

impl<'a> DrawableComponent for TextInputComponent<'a> {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		rect: Rect,
	) -> Result<()> {
		// this should always be true since draw should only be being called
		// is control is visible
		if let Some(ta) = &self.textarea {
			let area = if self.embed {
				rect
			} else {
				match self.input_type {
					InputType::Multiline => {
						let area =
							ui::centered_rect(60, 20, f.size());
						ui::rect_inside(
							Size::new(10, 3),
							f.size().into(),
							area,
						)
					}
					_ => ui::centered_rect_absolute(32, 3, f.size()),
				}
			};

			f.render_widget(Clear, area);

			f.render_widget(ta.widget(), area);

			if self.show_char_count {
				self.draw_char_count(f, area);
			}

			self.current_area.set(area);
		}
		Ok(())
	}
}

impl<'a> Component for TextInputComponent<'a> {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		_force_all: bool,
	) -> CommandBlocking {
		out.push(
			CommandInfo::new(
				strings::commands::close_popup(&self.key_config),
				true,
				self.is_visible(),
			)
			.order(1),
		);
		visibility_blocking(self)
	}

	fn event(&mut self, ev: &Event) -> Result<EventState> {
		if let Some(ta) = &mut self.textarea {
			if let Event::Key(e) = ev {
				if key_match(e, self.key_config.keys.exit_popup) {
					self.hide();
					return Ok(EventState::Consumed);
				}

				// So here all 'known' special keys for any textinput call are filtered out

				if key_match(e, self.key_config.keys.enter)
					|| key_match(
						e,
						self.key_config.keys.toggle_verify,
					) || key_match(
					e,
					self.key_config.keys.commit_amend,
				) || key_match(
					e,
					self.key_config.keys.open_commit_editor,
				) || key_match(
					e,
					self.key_config.keys.commit_history_next,
				) {
					return Ok(EventState::NotConsumed);
				}
				let input = Input::from(ev.clone());

				/*
				here we do key handling rather than passing it to textareas input function
				- so that we know which keys were handled and which were not
				- to get fine control over what each key press does
				- allow for key mapping based off key config....
				  but in fact the original textinput ignored all key bindings, up,down,right,....
				  so they are also ignored here

				*/
				match input {
					Input {
						key: Key::Char('m'),
						ctrl: true,
						alt: false,
					}
					| Input {
						key: Key::Char('\n' | '\r'),
						ctrl: false,
						alt: false,
					}
					/*  do not expect to see this one 
					 but it can be remapped
					 ctrl-Enter and shift-enter get here
					 */
					| Input {
						key: Key::Enter, ..
					} => {
						// prevent new lines in case of non multiline
						// password is assumed single line too
						if self.input_type == InputType::Multiline{
							ta.insert_newline();
						} else {
							return Ok(EventState::NotConsumed);
						}
					}
					Input {
						key: Key::Char(c),
						ctrl: false,
						alt: false,
					} => ta.insert_char(c),

					Input {
						key: Key::Tab,
						ctrl: false,
						alt: false,
					} => {
						ta.insert_tab();
						()
					}
					Input {
						key: Key::Char('h'),
						ctrl: true,
						alt: false,
					}
					| Input {
						key: Key::Backspace,
						ctrl: false,
						alt: false,
					} => {
						ta.delete_char();
						()
					}
					Input {
						key: Key::Char('d'),
						ctrl: true,
						alt: false,
					}
					| Input {
						key: Key::Delete,
						ctrl: false,
						alt: false,
					} => {
						ta.delete_next_char();
						()
					}
					Input {
						key: Key::Char('k'),
						ctrl: true,
						alt: false,
					} => {
						ta.delete_line_by_end();
						()
					}
					Input {
						key: Key::Char('j'),
						ctrl: true,
						alt: false,
					} => {
						ta.delete_line_by_head();
						()
					}
					Input {
						key: Key::Char('w'),
						ctrl: true,
						alt: false,
					}
					| Input {
						key: Key::Char('h'),
						ctrl: false,
						alt: true,
					}
					| Input {
						key: Key::Backspace,
						ctrl: false,
						alt: true,
					} => {
						ta.delete_word();
						()
					}
					Input {
						key: Key::Delete,
						ctrl: false,
						alt: true,
					}
					| Input {
						key: Key::Char('d'),
						ctrl: false,
						alt: true,
					} => {
						ta.delete_next_word();
						()
					}
					Input {
						key: Key::Char('n'),
						ctrl: true,
						alt: false,
					}
					| Input {
						key: Key::Down,
						ctrl: false,
						alt: false,
					} => ta.move_cursor(CursorMove::Down),

					Input {
						key: Key::Char('p'),
						ctrl: true,
						alt: false,
					}
					| Input {
						key: Key::Up,
						ctrl: false,
						alt: false,
					} => ta.move_cursor(CursorMove::Up),
					Input {
						key: Key::Char('f'),
						ctrl: true,
						alt: false,
					} |
					 Input {
						key: Key::Right,
						ctrl: false,
						alt: false,
					} => ta.move_cursor(CursorMove::Forward),
					Input {
						key: Key::Char('b'),
						ctrl: true,
						alt: false,
					}
					| Input {
						key: Key::Left,
						ctrl: false,
						alt: false,
					} => {
						ta.move_cursor(CursorMove::Back);
					}
					// normally picked up earlier as 'amend'
					 Input {
					 	key: Key::Char('a'),
					 	ctrl: true,
					 	alt: false,
					 }
					 |
					Input { key: Key::Home, .. }
					| Input {
						key: Key::Left | Key::Char('b'),
						ctrl: true,
						alt: true,
					} => {
						ta.move_cursor(CursorMove::Head);
					}
					// normally picked up earlier as 'invoke editor'
					Input {
					 	key: Key::Char('e'),
					 	ctrl: true,
					 	alt: false,
					 }
					 | Input { key: Key::End, .. }
					| Input {
						key: Key::Right | Key::Char('f'),
						ctrl: true,
						alt: true,
					} => {
						ta.move_cursor(CursorMove::End);
					}
					Input {
						key: Key::Char('<'),
						ctrl: false,
						alt: true,
					}
					| Input {
						key: Key::Up | Key::Char('p'),
						ctrl: true,
						alt: true,
					} => {
						ta.move_cursor(CursorMove::Top);
					}
					Input {
						key: Key::Char('>'),
						ctrl: false,
						alt: true,
					}
					| Input {
						key: Key::Down | Key::Char('n'),
						ctrl: true,
						alt: true,
					} => ta.move_cursor(CursorMove::Bottom),
					Input {
						key: Key::Char('f'),
						ctrl: false,
						alt: true,
					}
					| Input {
						key: Key::Right,
						ctrl: true,
						alt: false,
					} => ta.move_cursor(CursorMove::WordForward),

					Input {
						key: Key::Char('b'),
						ctrl: false,
						alt: true,
					}
					| Input {
						key: Key::Left,
						ctrl: true,
						alt: false,
					} => ta.move_cursor(CursorMove::WordBack),

					Input {
						key: Key::Char(']'),
						ctrl: false,
						alt: true,
					}
					| Input {
						key: Key::Char('n'),
						ctrl: false,
						alt: true,
					}
					| Input {
						key: Key::Down,
						ctrl: true,
						alt: false,
					} => ta.move_cursor(CursorMove::ParagraphForward),

					Input {
						key: Key::Char('['),
						ctrl: false,
						alt: true,
					}
					| Input {
						key: Key::Char('p'),
						ctrl: false,
						alt: true,
					}
					| Input {
						key: Key::Up,
						ctrl: true,
						alt: false,
					} => ta.move_cursor(CursorMove::ParagraphBack),

					Input {
						key: Key::Char('u'),
						ctrl: true,
						alt: false,
					} => {
						ta.undo();
						()
					}
					Input {
						key: Key::Char('r'),
						ctrl: true,
						alt: false,
					} => {
						ta.redo();
						()
					}
					Input {
						key: Key::Char('y'),
						ctrl: true,
						alt: false,
					} => {
						ta.paste();
						()
					}
					Input {
						key: Key::Char('v'),
						ctrl: true,
						alt: false,
					}
					| Input {
						key: Key::PageDown, ..
					} => ta.scroll(Scrolling::PageDown),

					Input {
						key: Key::Char('v'),
						ctrl: false,
						alt: true,
					}
					| Input {
						key: Key::PageUp, ..
					} => ta.scroll(Scrolling::PageUp),

					Input {
						key: Key::MouseScrollDown,
						..
					} => ta.scroll((1, 0)),

					Input {
						key: Key::MouseScrollUp,
						..
					} => ta.scroll((-1, 0)),

					_ => return Ok(EventState::NotConsumed),
				};
			}
			return Ok(EventState::Consumed);
		}
		Ok(EventState::NotConsumed)
	}
	/*
	  visible maps to textarea Option
	  None = > not visible
	  Some => visible
	*/
	fn is_visible(&self) -> bool {
		self.textarea.is_some()
	}

	fn hide(&mut self) {
		self.textarea = None;
	}

	fn show(&mut self) -> Result<()> {
		self.create_and_show();
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use ratatui::{style::Style, text::Span};

	#[test]
	fn test_smoke() {
		let mut comp = TextInputComponent::new(
			SharedTheme::default(),
			SharedKeyConfig::default(),
			"",
			"",
			false,
		);
		comp.create_and_show();
		comp.set_text(String::from("a\nb"));
		assert!(comp.is_visible());
		if let Some(ta) = &mut comp.textarea {
			assert_eq!(ta.cursor(), (0, 0));

			ta.move_cursor(CursorMove::Forward);
			assert_eq!(ta.cursor(), (0, 1));

			ta.move_cursor(CursorMove::Back);
			assert_eq!(ta.cursor(), (0, 0));
		}
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
		comp.create_and_show();
		comp.set_text(String::from("a"));
		assert!(comp.is_visible());
		if let Some(ta) = &mut comp.textarea {
			let txt = ta.lines();
			assert_eq!(txt[0].len(), 1);
			assert_eq!(txt[0].as_bytes()[0], 'a' as u8);
		}
	}

	#[test]
	fn test_multiline() {
		let mut comp = TextInputComponent::new(
			SharedTheme::default(),
			SharedKeyConfig::default(),
			"",
			"",
			false,
		);
		comp.create_and_show();
		comp.set_text(String::from("a\nb\nc"));
		assert!(comp.is_visible());
		if let Some(ta) = &mut comp.textarea {
			let txt = ta.lines();
			assert_eq!(txt[0], "a");
			assert_eq!(txt[1], "b");
			assert_eq!(txt[2], "c");
		}
	}

	#[test]
	fn test_next_word_position() {
		let mut comp = TextInputComponent::new(
			SharedTheme::default(),
			SharedKeyConfig::default(),
			"",
			"",
			false,
		);
		comp.create_and_show();
		comp.set_text(String::from("aa b;c"));
		assert!(comp.is_visible());
		if let Some(ta) = &mut comp.textarea {
			// from word start
			ta.move_cursor(CursorMove::Head);
			ta.move_cursor(CursorMove::WordForward);
			assert_eq!(ta.cursor(), (0, 3));
			// from inside start
			ta.move_cursor(CursorMove::Forward);
			ta.move_cursor(CursorMove::WordForward);
			assert_eq!(ta.cursor(), (0, 5));
			// to string end
			ta.move_cursor(CursorMove::Forward);
			ta.move_cursor(CursorMove::WordForward);
			assert_eq!(ta.cursor(), (0, 6));

			// from string end
			ta.move_cursor(CursorMove::Forward);
			let save_cursor = ta.cursor();
			ta.move_cursor(CursorMove::WordForward);
			assert_eq!(ta.cursor(), save_cursor);
		}
	}

	#[test]
	fn test_previous_word_position() {
		let mut comp = TextInputComponent::new(
			SharedTheme::default(),
			SharedKeyConfig::default(),
			"",
			"",
			false,
		);
		comp.create_and_show();
		comp.set_text(String::from(" a bb;c"));
		assert!(comp.is_visible());

		if let Some(ta) = &mut comp.textarea {
			// from string end
			ta.move_cursor(CursorMove::End);
			ta.move_cursor(CursorMove::WordBack);
			assert_eq!(ta.cursor(), (0, 6));
			// from inside word
			ta.move_cursor(CursorMove::Back);
			ta.move_cursor(CursorMove::WordBack);
			assert_eq!(ta.cursor(), (0, 3));
			// from word start
			ta.move_cursor(CursorMove::WordBack);
			assert_eq!(ta.cursor(), (0, 1));
			// to string start
			ta.move_cursor(CursorMove::WordBack);
			assert_eq!(ta.cursor(), (0, 0));
			// from string start
			let save_cursor = ta.cursor();
			ta.move_cursor(CursorMove::WordBack);

			assert_eq!(ta.cursor(), save_cursor);
		}
	}

	#[test]
	fn test_next_word_multibyte() {
		let mut comp = TextInputComponent::new(
			SharedTheme::default(),
			SharedKeyConfig::default(),
			"",
			"",
			false,
		);
		// should emojis be word boundaries or not?
		// various editors (vs code, vim) do not agree with the
		// behavhior of the original textinput here.
		//
		// tui-textarea agrees with them.
		// So these tests are changed to match that behavior
		// FYI: this line is "a à ❤ab🤯 a"

		//              "01245       89A        EFG"
		let text = dbg!("a à \u{2764}ab\u{1F92F} a");
		comp.create_and_show();
		comp.set_text(String::from(text));
		assert!(comp.is_visible());

		if let Some(ta) = &mut comp.textarea {
			ta.move_cursor(CursorMove::Head);
			ta.move_cursor(CursorMove::WordForward);
			assert_eq!(ta.cursor(), (0, 2));
			ta.move_cursor(CursorMove::WordForward);
			assert_eq!(ta.cursor(), (0, 4));
			ta.move_cursor(CursorMove::WordForward);
			assert_eq!(ta.cursor(), (0, 9));
			ta.move_cursor(CursorMove::WordForward);
			assert_eq!(ta.cursor(), (0, 10));
			let save_cursor = ta.cursor();
			ta.move_cursor(CursorMove::WordForward);
			assert_eq!(ta.cursor(), save_cursor);

			ta.move_cursor(CursorMove::End);
			ta.move_cursor(CursorMove::WordBack);
			assert_eq!(ta.cursor(), (0, 9));
			ta.move_cursor(CursorMove::WordBack);
			assert_eq!(ta.cursor(), (0, 4));
			ta.move_cursor(CursorMove::WordBack);
			assert_eq!(ta.cursor(), (0, 2));
			ta.move_cursor(CursorMove::WordBack);
			assert_eq!(ta.cursor(), (0, 0));
			let save_cursor = ta.cursor();
			ta.move_cursor(CursorMove::WordBack);
			assert_eq!(ta.cursor(), save_cursor);
		}
	}
}
