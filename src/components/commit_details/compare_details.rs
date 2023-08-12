use std::borrow::Cow;

use crate::{
	components::{
		commit_details::style::{style_detail, Detail},
		dialog_paragraph,
		utils::time_to_string,
		CommandBlocking, CommandInfo, Component, DrawableComponent,
		EventState,
	},
	strings::{self},
	ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::sync::{self, CommitDetails, CommitId, RepoPathRef};
use crossterm::event::Event;
use ratatui::{
	backend::Backend,
	layout::{Constraint, Direction, Layout, Rect},
	text::{Line, Span, Text},
	Frame,
};

pub struct CompareDetailsComponent {
	repo: RepoPathRef,
	data: Option<(CommitDetails, CommitDetails)>,
	theme: SharedTheme,
	focused: bool,
}

impl CompareDetailsComponent {
	///
	pub const fn new(
		repo: RepoPathRef,
		theme: SharedTheme,
		focused: bool,
	) -> Self {
		Self {
			data: None,
			theme,
			focused,
			repo,
		}
	}

	pub fn set_commits(&mut self, ids: Option<(CommitId, CommitId)>) {
		self.data = ids.and_then(|ids| {
			let c1 =
				sync::get_commit_details(&self.repo.borrow(), ids.0)
					.ok();
			let c2 =
				sync::get_commit_details(&self.repo.borrow(), ids.1)
					.ok();

			c1.and_then(|c1| {
				c2.map(|c2| {
					if c1.author.time < c2.author.time {
						(c1, c2)
					} else {
						(c2, c1)
					}
				})
			})
		});
	}

	#[allow(unstable_name_collisions)]
	fn get_commit_text(&self, data: &CommitDetails) -> Vec<Line> {
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

		res.push(Line::from(vec![
			style_detail(&self.theme, &Detail::Message),
			Span::styled(
				Cow::from(
					data.message
						.as_ref()
						.map(|msg| msg.subject.clone())
						.unwrap_or_default(),
				),
				self.theme.text(true, false),
			),
		]));

		res
	}
}

impl DrawableComponent for CompareDetailsComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		rect: Rect,
	) -> Result<()> {
		let chunks = Layout::default()
			.direction(Direction::Vertical)
			.constraints(
				[Constraint::Length(5), Constraint::Length(5)]
					.as_ref(),
			)
			.split(rect);

		if let Some(data) = &self.data {
			f.render_widget(
				dialog_paragraph(
					&strings::commit::compare_details_info_title(
						true,
						data.0.short_hash(),
					),
					Text::from(self.get_commit_text(&data.0)),
					&self.theme,
					false,
				),
				chunks[0],
			);

			f.render_widget(
				dialog_paragraph(
					&strings::commit::compare_details_info_title(
						false,
						data.1.short_hash(),
					),
					Text::from(self.get_commit_text(&data.1)),
					&self.theme,
					false,
				),
				chunks[1],
			);
		}

		Ok(())
	}
}

impl Component for CompareDetailsComponent {
	fn commands(
		&self,
		_out: &mut Vec<CommandInfo>,
		_force_all: bool,
	) -> CommandBlocking {
		CommandBlocking::PassingOn
	}

	fn event(&mut self, _event: &Event) -> Result<EventState> {
		Ok(EventState::NotConsumed)
	}

	fn focused(&self) -> bool {
		self.focused
	}

	fn focus(&mut self, focus: bool) {
		self.focused = focus;
	}
}
