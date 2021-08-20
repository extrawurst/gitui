use crate::{
	components::{
		dialog_paragraph, CommandBlocking, CommandInfo, Component,
		DrawableComponent, EventState,
	},
	keys::SharedKeyConfig,
	strings::{self},
	ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
	sync::{self, CommitDetails, CommitId},
	CWD,
};
use crossterm::event::Event;
use tui::{
	backend::Backend,
	layout::{Constraint, Direction, Layout, Rect},
	text::Text,
	Frame,
};

pub struct CompareDetailsComponent {
	data: Option<(CommitDetails, CommitDetails)>,
	theme: SharedTheme,
	focused: bool,
	key_config: SharedKeyConfig,
}

impl CompareDetailsComponent {
	///
	pub const fn new(
		theme: SharedTheme,
		key_config: SharedKeyConfig,
		focused: bool,
	) -> Self {
		Self {
			data: None,
			theme,
			focused,
			key_config,
		}
	}

	pub fn set_commits(&mut self, ids: Option<(CommitId, CommitId)>) {
		self.data = if let Some(ids) = ids {
			let c1 = sync::get_commit_details(CWD, ids.0).ok();
			let c2 = sync::get_commit_details(CWD, ids.1).ok();

			c1.map(|c1| c2.map(|c2| (c1, c2))).flatten()
		} else {
			None
		};
	}

	// fn style_detail(&self, field: &Detail) -> Span {
	// 	match field {
	// 		Detail::Author => Span::styled(
	// 			Cow::from(strings::commit::details_author(
	// 				&self.key_config,
	// 			)),
	// 			self.theme.text(false, false),
	// 		),
	// 		Detail::Date => Span::styled(
	// 			Cow::from(strings::commit::details_date(
	// 				&self.key_config,
	// 			)),
	// 			self.theme.text(false, false),
	// 		),
	// 		Detail::Commiter => Span::styled(
	// 			Cow::from(strings::commit::details_committer(
	// 				&self.key_config,
	// 			)),
	// 			self.theme.text(false, false),
	// 		),
	// 		Detail::Sha => Span::styled(
	// 			Cow::from(strings::commit::details_tags(
	// 				&self.key_config,
	// 			)),
	// 			self.theme.text(false, false),
	// 		),
	// 	}
	// }

	// #[allow(unstable_name_collisions)]
	// fn get_text_info(&self) -> Vec<Spans> {
	// 	if let Some(ref data) = self.data {
	// 		let mut res = vec![
	// 			Spans::from(vec![
	// 				self.style_detail(&Detail::Author),
	// 				Span::styled(
	// 					Cow::from(format!(
	// 						"{} <{}>",
	// 						data.author.name, data.author.email
	// 					)),
	// 					self.theme.text(true, false),
	// 				),
	// 			]),
	// 			Spans::from(vec![
	// 				self.style_detail(&Detail::Date),
	// 				Span::styled(
	// 					Cow::from(time_to_string(
	// 						data.author.time,
	// 						false,
	// 					)),
	// 					self.theme.text(true, false),
	// 				),
	// 			]),
	// 		];

	// 		if let Some(ref committer) = data.committer {
	// 			res.extend(vec![
	// 				Spans::from(vec![
	// 					self.style_detail(&Detail::Commiter),
	// 					Span::styled(
	// 						Cow::from(format!(
	// 							"{} <{}>",
	// 							committer.name, committer.email
	// 						)),
	// 						self.theme.text(true, false),
	// 					),
	// 				]),
	// 				Spans::from(vec![
	// 					self.style_detail(&Detail::Date),
	// 					Span::styled(
	// 						Cow::from(time_to_string(
	// 							committer.time,
	// 							false,
	// 						)),
	// 						self.theme.text(true, false),
	// 					),
	// 				]),
	// 			]);
	// 		}

	// 		res.push(Spans::from(vec![
	// 			Span::styled(
	// 				Cow::from(strings::commit::details_sha(
	// 					&self.key_config,
	// 				)),
	// 				self.theme.text(false, false),
	// 			),
	// 			Span::styled(
	// 				Cow::from(data.hash.clone()),
	// 				self.theme.text(true, false),
	// 			),
	// 		]));

	// 		if !self.tags.is_empty() {
	// 			res.push(Spans::from(
	// 				self.style_detail(&Detail::Sha),
	// 			));

	// 			res.push(Spans::from(
	// 				self.tags
	// 					.iter()
	// 					.map(|tag| {
	// 						Span::styled(
	// 							Cow::from(tag),
	// 							self.theme.text(true, false),
	// 						)
	// 					})
	// 					.intersperse(Span::styled(
	// 						Cow::from(","),
	// 						self.theme.text(true, false),
	// 					))
	// 					.collect::<Vec<Span>>(),
	// 			));
	// 		}

	// 		res
	// 	} else {
	// 		vec![]
	// 	}
	// }
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
				[Constraint::Length(8), Constraint::Min(10)].as_ref(),
			)
			.split(rect);

		f.render_widget(
			dialog_paragraph(
				&strings::commit::details_info_title(
					&self.key_config,
				),
				Text::from(""),
				&self.theme,
				false,
			),
			chunks[0],
		);

		// f.render_widget(
		// 	dialog_paragraph(
		// 		&format!(
		// 			"{} {}",
		// 			strings::commit::details_message_title(
		// 				&self.key_config,
		// 			),
		// 			if !self.focused && can_scroll {
		// 				CANSCROLL_STRING
		// 			} else {
		// 				EMPTY_STRING
		// 			}
		// 		),
		// 		Text::from(self.get_wrapped_text_message(
		// 			width as usize,
		// 			height as usize,
		// 		)),
		// 		&self.theme,
		// 		self.focused,
		// 	),
		// 	chunks[1],
		// );

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

	fn event(&mut self, _event: Event) -> Result<EventState> {
		Ok(EventState::NotConsumed)
	}

	fn focused(&self) -> bool {
		self.focused
	}

	fn focus(&mut self, focus: bool) {
		self.focused = focus;
	}
}
