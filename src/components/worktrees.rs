use anyhow::Result;
use asyncgit::sync::{RepoPathRef, worktrees};
use tui::{backend::Backend, Frame, layout::Rect, widgets::{Block, Borders}, text::Span, style::Style};

use crate::ui::{style::SharedTheme, self};

use super::DrawableComponent;


pub struct WorkTreesComponent {
	repo: RepoPathRef,
	visible: bool,
    theme: SharedTheme,
}

impl WorkTreesComponent {
	///
	pub fn new(
		repo: RepoPathRef,
        theme: SharedTheme,
	) -> Self {
		Self {
			repo,
			visible: false,
            theme,
		}
	}

	fn is_visible(&self) -> bool {
		self.visible
	}

}

impl DrawableComponent for WorkTreesComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		area: Rect,
	) -> Result<()> {
        log::trace!("delete me later {:?}", self.repo);
		if self.is_visible() {
		}
        worktrees(&self.repo.borrow())?;
        let items = vec![Span::styled("pls", Style::default())].into_iter();
		ui::draw_list_block(
			f,
			area,
			Block::default()
				.title(Span::styled(
					"Hello World".to_string(),
					self.theme.title(true),
				))
				.borders(Borders::ALL)
				.border_style(self.theme.block(true)),
			items,
		);
		Ok(())
	}
}

