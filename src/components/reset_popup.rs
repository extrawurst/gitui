#![allow(dead_code)]

use super::{
	visibility_blocking, CommandBlocking, CommandInfo, Component,
	DrawableComponent, EventState,
};
use crate::{
	keys::{key_match, SharedKeyConfig},
	strings,
	ui::{self, style::SharedTheme},
};
use anyhow::Result;
use asyncgit::sync::{CommitId, RepoPath, RepoPathRef, ResetType};
use crossterm::event::Event;
use tui::{
	backend::Backend,
	layout::{Alignment, Rect},
	text::{Span, Spans},
	widgets::{Block, Borders, Clear, Paragraph},
	Frame,
};

const fn type_to_string(kind: ResetType) -> &'static str {
	match kind {
		ResetType::Soft => "🟢 Soft",
		ResetType::Mixed => "🟡 Mixed",
		ResetType::Hard => "🔴 Hard",
	}
}

pub struct ResetPopupComponent {
	repo: RepoPath,
	commit: Option<CommitId>,
	kind: ResetType,
	visible: bool,
	key_config: SharedKeyConfig,
	theme: SharedTheme,
}

impl ResetPopupComponent {
	///
	pub fn new(
		repo: &RepoPathRef,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			repo: repo.borrow().clone(),
			commit: None,
			kind: ResetType::Soft,
			visible: false,
			key_config,
			theme,
		}
	}

	fn get_text(&self, _width: u16) -> Vec<Spans> {
		let mut txt: Vec<Spans> = Vec::with_capacity(10);

		txt.push(Spans::from(vec![
			Span::styled(
				String::from("Branch: "),
				self.theme.text(true, false),
			),
			Span::styled("master", self.theme.branch(false, true)),
		]));

		txt.push(Spans::from(vec![
			Span::styled(
				String::from("Reset to: "),
				self.theme.text(true, false),
			),
			Span::styled(
				self.commit
					.map(|c| c.to_string())
					.unwrap_or_default(),
				self.theme.commit_hash(false),
			),
		]));

		txt.push(Spans::from(vec![
			Span::styled(
				String::from("How: "),
				self.theme.text(true, false),
			),
			Span::styled(
				type_to_string(self.kind),
				self.theme.text(true, true),
			),
		]));

		txt
	}

	///
	pub fn open(&mut self, id: CommitId) -> Result<()> {
		self.show()?;

		self.commit = Some(id);

		Ok(())
	}

	fn reset(&mut self) -> Result<()> {
		if let Some(id) = self.commit {
			asyncgit::sync::reset_repo(&self.repo, id, self.kind)?;
		}

		self.hide();

		Ok(())
	}
}

impl DrawableComponent for ResetPopupComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		area: Rect,
	) -> Result<()> {
		if self.is_visible() {
			const SIZE: (u16, u16) = (55, 5);
			let area =
				ui::centered_rect_absolute(SIZE.0, SIZE.1, area);

			let width = area.width;

			f.render_widget(Clear, area);
			f.render_widget(
				Paragraph::new(self.get_text(width))
					.block(
						Block::default()
							.borders(Borders::ALL)
							.title(Span::styled(
								"Reset",
								self.theme.title(true),
							))
							.border_style(self.theme.block(true)),
					)
					.alignment(Alignment::Left),
				area,
			);
		}

		Ok(())
	}
}

impl Component for ResetPopupComponent {
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
					strings::commands::reset_commit(&self.key_config),
					true,
					true,
				)
				.order(1),
			);
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
				} else if key_match(key, self.key_config.keys.enter) {
					self.reset()?;
				}
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
