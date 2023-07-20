use crate::{
	components::{
		commit_details::style::style_detail,
		dialog_paragraph,
		utils::{scroll_vertical::VerticalScroll, time_to_string},
		CommandBlocking, CommandInfo, Component, DrawableComponent,
		EventState, ScrollType,
	},
	keys::{key_match, SharedKeyConfig},
	strings::{self, order},
	ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::sync::{
	self, CommitDetails, CommitId, CommitMessage, RepoPathRef, Tag,
};
use crossterm::event::Event;
use ratatui::{
	backend::Backend,
	layout::{Constraint, Direction, Layout, Rect},
	style::{Modifier, Style},
	text::{Line, Span, Text},
	Frame,
};
use std::clone::Clone;
use std::{borrow::Cow, cell::Cell};
use sync::CommitTags;

use super::style::Detail;

pub struct DetailsComponent {
	repo: RepoPathRef,
	data: Option<CommitDetails>,
	tags: Vec<Tag>,
	theme: SharedTheme,
	focused: bool,
	current_width: Cell<u16>,
	scroll: VerticalScroll,
	scroll_to_bottom_next_draw: Cell<bool>,
	key_config: SharedKeyConfig,
}

type WrappedCommitMessage<'a> =
	(Vec<Cow<'a, str>>, Vec<Cow<'a, str>>);

impl DetailsComponent {
	///
	pub const fn new(
		repo: RepoPathRef,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
		focused: bool,
	) -> Self {
		Self {
			repo,
			data: None,
			tags: Vec::new(),
			theme,
			focused,
			scroll_to_bottom_next_draw: Cell::new(false),
			current_width: Cell::new(0),
			scroll: VerticalScroll::new(),
			key_config,
		}
	}

	pub fn set_commit(
		&mut self,
		id: Option<CommitId>,
		tags: Option<CommitTags>,
	) {
		self.tags.clear();

		self.data = id.and_then(|id| {
			sync::get_commit_details(&self.repo.borrow(), id).ok()
		});

		self.scroll.reset();

		if let Some(tags) = tags {
			self.tags.extend(tags);
		}
	}

	fn wrap_commit_details<'a>(
		message: &CommitMessage,
		width: usize,
		title_buffer: &'a mut [u8],
		message_buffer: &'a mut [u8],
	) -> WrappedCommitMessage<'a> {
		let _ = bwrap::Wrapper::new(
			&message.subject,
			width,
			title_buffer,
		)
		.unwrap()
		.wrap();
		let wrapped_title = std::str::from_utf8(title_buffer)
			.unwrap()
			.split('\n')
			.map(|s| Cow::Borrowed(s.trim_null()))
			.collect();

		if let Some(ref body) = message.body {
			let _ = bwrap::Wrapper::new(body, width, message_buffer)
				.unwrap()
				.wrap();
			let wrapped_message: Vec<Cow<'_, str>> =
				std::str::from_utf8(message_buffer)
					.unwrap()
					.split('\n')
					.map(|s| Cow::Borrowed(s.trim_null()))
					.collect();

			(wrapped_title, wrapped_message)
		} else {
			(wrapped_title, vec![])
		}
	}

	fn get_wrapped_lines<'a>(
		data: &Option<CommitDetails>,
		width: usize,
		title_buffer: &'a mut [u8],
		message_buffer: &'a mut [u8],
	) -> WrappedCommitMessage<'a> {
		if let Some(ref data) = data {
			if let Some(ref message) = data.message {
				return Self::wrap_commit_details(
					message,
					width,
					title_buffer,
					message_buffer,
				);
			}
		}

		(vec![], vec![])
	}

	fn get_number_of_lines(
		details: &Option<CommitDetails>,
		width: usize,
	) -> usize {
		if let Some(ref data) = details {
			if let Some(ref message) = data.message {
				let title_len = message.subject.len();
				let mut title_buffer =
					vec![0; title_len + title_len / width + 1];
				let message_len = match message.body {
					Some(ref body) => body.len(),
					None => 0usize,
				};
				let mut message_buffer =
					vec![0; message_len + message_len / width + 1];
				let (wrapped_title, wrapped_message) =
					Self::wrap_commit_details(
						message,
						width,
						&mut title_buffer,
						&mut message_buffer,
					);
				return wrapped_title.len() + wrapped_message.len();
			}
		}
		return 0usize;
	}

	fn get_theme_for_line(&self, bold: bool) -> Style {
		if bold {
			self.theme.text(true, false).add_modifier(Modifier::BOLD)
		} else {
			self.theme.text(true, false)
		}
	}

	fn get_wrapped_text_message<'a>(
		&'a self,
		width: usize,
		height: usize,
		title_buffer: &'a mut [u8],
		message_buffer: &'a mut [u8],
	) -> Vec<Line> {
		let (wrapped_title, wrapped_message) =
			Self::get_wrapped_lines(
				&self.data,
				width,
				title_buffer,
				message_buffer,
			);

		[&wrapped_title[..], &wrapped_message[..]]
			.concat()
			.iter()
			.enumerate()
			.skip(self.scroll.get_top())
			.take(height)
			.map(|(i, line)| {
				Line::from(vec![Span::styled(
					line.clone(),
					self.get_theme_for_line(i < wrapped_title.len()),
				)])
			})
			.collect()
	}

	#[allow(unstable_name_collisions, clippy::too_many_lines)]
	fn get_text_info(&self) -> Vec<Line> {
		self.data.as_ref().map_or_else(Vec::new, |data| {
			let mut res = vec![
				Line::from(vec![
					style_detail(&self.theme, &Detail::Author),
					Span::styled(
						Cow::from(format!(
							"{} <{}>",
							data.author.name, data.author.email
						)),
						self.theme.text(true, false),
					),
				]),
				Line::from(vec![
					style_detail(&self.theme, &Detail::Date),
					Span::styled(
						Cow::from(time_to_string(
							data.author.time,
							false,
						)),
						self.theme.text(true, false),
					),
				]),
			];

			if let Some(ref committer) = data.committer {
				res.extend(vec![
					Line::from(vec![
						style_detail(&self.theme, &Detail::Commiter),
						Span::styled(
							Cow::from(format!(
								"{} <{}>",
								committer.name, committer.email
							)),
							self.theme.text(true, false),
						),
					]),
					Line::from(vec![
						style_detail(&self.theme, &Detail::Date),
						Span::styled(
							Cow::from(time_to_string(
								committer.time,
								false,
							)),
							self.theme.text(true, false),
						),
					]),
				]);
			}

			res.push(Line::from(vec![
				Span::styled(
					Cow::from(strings::commit::details_sha()),
					self.theme.text(false, false),
				),
				Span::styled(
					Cow::from(data.hash.clone()),
					self.theme.text(true, false),
				),
			]));

			if !self.tags.is_empty() {
				res.push(Line::from(style_detail(
					&self.theme,
					&Detail::Sha,
				)));

				res.push(Line::from(
					itertools::Itertools::intersperse(
						self.tags.iter().map(|tag| {
							Span::styled(
								Cow::from(&tag.name),
								self.theme.text(true, false),
							)
						}),
						Span::styled(
							Cow::from(","),
							self.theme.text(true, false),
						),
					)
					.collect::<Vec<Span>>(),
				));
			}

			res
		})
	}

	fn move_scroll_top(&mut self, move_type: ScrollType) -> bool {
		if self.data.is_some() {
			self.scroll.move_top(move_type)
		} else {
			false
		}
	}
}

impl DrawableComponent for DetailsComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		rect: Rect,
	) -> Result<()> {
		const CANSCROLL_STRING: &str = "[\u{2026}]";
		const EMPTY_STRING: &str = "";

		let chunks = Layout::default()
			.direction(Direction::Vertical)
			.constraints(
				[Constraint::Length(8), Constraint::Min(10)].as_ref(),
			)
			.split(rect);

		f.render_widget(
			dialog_paragraph(
				&strings::commit::details_info_title(
					&self.key_config,
				),
				Text::from(self.get_text_info()),
				&self.theme,
				false,
			),
			chunks[0],
		);

		// We have to take the border into account which is one character on
		// each side.
		let border_width: u16 = 2;

		let width = chunks[1].width.saturating_sub(border_width);
		let height = chunks[1].height.saturating_sub(border_width);

		self.current_width.set(width);

		let number_of_lines =
			Self::get_number_of_lines(&self.data, usize::from(width));

		self.scroll.update_no_selection(
			number_of_lines,
			usize::from(height),
		);

		if self.scroll_to_bottom_next_draw.get() {
			self.scroll.move_top(ScrollType::End);
			self.scroll_to_bottom_next_draw.set(false);
		}

		let can_scroll = usize::from(height) < number_of_lines;

		let (title_len, message_len) = match self.data {
			None => (0usize, 0usize),
			Some(ref data) => match data.message {
				None => (0usize, 0usize),
				Some(ref message) => {
					let title_len = message.subject.len();
					let message_len = match message.body {
						None => 0usize,
						Some(ref body) => body.len(),
					};
					(title_len, message_len)
				}
			},
		};
		let mut title_buffer =
			vec![0u8; title_len + title_len / usize::from(width) + 1];
		let mut message_buffer =
			vec![
				0u8;
				message_len + message_len / usize::from(width) + 1
			];
		f.render_widget(
			dialog_paragraph(
				&format!(
					"{} {}",
					strings::commit::details_message_title(
						&self.key_config,
					),
					if !self.focused && can_scroll {
						CANSCROLL_STRING
					} else {
						EMPTY_STRING
					}
				),
				Text::from(self.get_wrapped_text_message(
					width as usize,
					height as usize,
					&mut title_buffer,
					&mut message_buffer,
				)),
				&self.theme,
				self.focused,
			),
			chunks[1],
		);

		if self.focused {
			self.scroll.draw(f, chunks[1], &self.theme);
		}

		Ok(())
	}
}

impl Component for DetailsComponent {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		let width = usize::from(self.current_width.get());
		let number_of_lines =
			Self::get_number_of_lines(&self.data, width);

		out.push(
			CommandInfo::new(
				strings::commands::navigate_commit_message(
					&self.key_config,
				),
				number_of_lines > 0,
				self.focused || force_all,
			)
			.order(order::NAV),
		);

		CommandBlocking::PassingOn
	}

	fn event(&mut self, event: &Event) -> Result<EventState> {
		if self.focused {
			if let Event::Key(e) = event {
				return Ok(
					if key_match(e, self.key_config.keys.move_up) {
						self.move_scroll_top(ScrollType::Up).into()
					} else if key_match(
						e,
						self.key_config.keys.move_down,
					) {
						self.move_scroll_top(ScrollType::Down).into()
					} else if key_match(e, self.key_config.keys.home)
						|| key_match(e, self.key_config.keys.shift_up)
					{
						self.move_scroll_top(ScrollType::Home).into()
					} else if key_match(e, self.key_config.keys.end)
						|| key_match(
							e,
							self.key_config.keys.shift_down,
						) {
						self.move_scroll_top(ScrollType::End).into()
					} else {
						EventState::NotConsumed
					},
				);
			}
		}

		Ok(EventState::NotConsumed)
	}

	fn focused(&self) -> bool {
		self.focused
	}

	fn focus(&mut self, focus: bool) {
		if focus {
			self.scroll_to_bottom_next_draw.set(true);
		} else {
			self.scroll.reset();
		}

		self.focused = focus;
	}
}

trait NullTrim {
	fn trim_null(&self) -> &Self;
}

impl NullTrim for str {
	fn trim_null(&self) -> &Self {
		let end = match self.find('\0') {
			None => self.len(),
			Some(pos) => pos,
		};
		&self[..end]
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn get_wrapped_lines<'a>(
		message: &CommitMessage,
		width: usize,
		title_buffer: &'a mut [u8],
		message_buffer: &'a mut [u8],
	) -> Vec<Cow<'a, str>> {
		let (wrapped_title, wrapped_message) =
			DetailsComponent::wrap_commit_details(
				message,
				width,
				title_buffer,
				message_buffer,
			);

		[&wrapped_title[..], &wrapped_message[..]].concat()
	}

	#[test]
	fn test_textwrap() {
		let message = CommitMessage::from("Commit message");
		let mut title_buffer = vec![0; 32];
		let mut message_buffer = vec![0; 32];

		assert_eq!(
			get_wrapped_lines(
				&message,
				7,
				&mut title_buffer,
				&mut message_buffer
			),
			vec!["Commit", "message"]
		);
		assert_eq!(
			get_wrapped_lines(
				&message,
				14,
				&mut title_buffer,
				&mut message_buffer
			),
			vec!["Commit message"]
		);

		let message_with_newline =
			CommitMessage::from("Commit message\n");

		assert_eq!(
			get_wrapped_lines(
				&message_with_newline,
				7,
				&mut title_buffer,
				&mut message_buffer
			),
			vec!["Commit", "message"]
		);
		assert_eq!(
			get_wrapped_lines(
				&message_with_newline,
				14,
				&mut title_buffer,
				&mut message_buffer
			),
			vec!["Commit message"]
		);

		let message_with_body = CommitMessage::from(
			"Commit message\nFirst line\nSecond line",
		);

		assert_eq!(
			get_wrapped_lines(
				&message_with_body,
				7,
				&mut title_buffer,
				&mut message_buffer
			),
			vec![
				"Commit", "message", "First", "line", "Second",
				"line"
			]
		);
		assert_eq!(
			get_wrapped_lines(
				&message_with_body,
				14,
				&mut title_buffer,
				&mut message_buffer
			),
			vec!["Commit message", "First line", "Second line"]
		);
	}
}

#[cfg(test)]
mod test_line_count {
	use super::*;

	#[test]
	fn test_smoke() {
		let commit = CommitDetails {
			message: Some(CommitMessage {
				subject: String::from("subject line"),
				body: Some(String::from("body lone")),
			}),
			..CommitDetails::default()
		};
		let lines = DetailsComponent::get_number_of_lines(
			&Some(commit.clone()),
			50,
		);
		assert_eq!(lines, 2);

		let lines =
			DetailsComponent::get_number_of_lines(&Some(commit), 8);
		assert_eq!(lines, 4);
	}
}
