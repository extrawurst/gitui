use anyhow::Result;
use asyncgit::sync::{RepoPathRef, WorkTree};
use tui::{backend::Backend, Frame, layout::Rect, widgets::{Block, Borders}, text::Span, style::Style};

use crate::ui::{style::SharedTheme, self};

use super::DrawableComponent;


pub struct WorkTreesComponent {
	repo: RepoPathRef,
	visible: bool,
    theme: SharedTheme,
    worktrees: Vec<WorkTree>,
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
            worktrees: Vec::new(),
		}
	}

    pub fn set_worktrees(&mut self, worktrees: Vec<WorkTree>) -> Result<()> {
        self.worktrees = worktrees;
        Ok(())
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
        let items = self.worktrees.iter().map(|w| Span::styled(w.name.clone(), Style::default()));
        //let items = vec![Span::styled("pls", Style::default())].into_iter();
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

