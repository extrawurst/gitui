mod reflow;
mod scrollbar;
mod scrolllist;
mod stateful_paragraph;
pub mod style;
mod syntax_text;

use filetreelist::MoveSelection;
pub use scrollbar::draw_scrollbar;
pub use scrolllist::{draw_list, draw_list_block};
pub use stateful_paragraph::{
	ParagraphState, ScrollPos, StatefulParagraph,
};
pub use syntax_text::{AsyncSyntaxJob, SyntaxText};
use tui::layout::{Constraint, Direction, Layout, Rect};

use crate::keys::SharedKeyConfig;

/// return the scroll position (line) necessary to have the `selection` in view if it is not already
pub const fn calc_scroll_top(
	current_top: usize,
	height_in_lines: usize,
	selection: usize,
) -> usize {
	if current_top + height_in_lines <= selection {
		selection.saturating_sub(height_in_lines) + 1
	} else if current_top > selection {
		selection
	} else {
		current_top
	}
}

/// ui component size representation
#[derive(Copy, Clone)]
pub struct Size {
	pub width: u16,
	pub height: u16,
}

impl Size {
	pub const fn new(width: u16, height: u16) -> Self {
		Self { width, height }
	}
}

impl From<Rect> for Size {
	fn from(r: Rect) -> Self {
		Self {
			width: r.width,
			height: r.height,
		}
	}
}

/// use layouts to create a rects that
/// centers inside `r` and sizes `percent_x`/`percent_x` of `r`
pub fn centered_rect(
	percent_x: u16,
	percent_y: u16,
	r: Rect,
) -> Rect {
	let popup_layout = Layout::default()
		.direction(Direction::Vertical)
		.constraints(
			[
				Constraint::Percentage((100 - percent_y) / 2),
				Constraint::Percentage(percent_y),
				Constraint::Percentage((100 - percent_y) / 2),
			]
			.as_ref(),
		)
		.split(r);

	Layout::default()
		.direction(Direction::Horizontal)
		.constraints(
			[
				Constraint::Percentage((100 - percent_x) / 2),
				Constraint::Percentage(percent_x),
				Constraint::Percentage((100 - percent_x) / 2),
			]
			.as_ref(),
		)
		.split(popup_layout[1])[1]
}

/// makes sure Rect `r` at least stays as big as min and not bigger than max
pub fn rect_inside(min: Size, max: Size, r: Rect) -> Rect {
	let new_width = r.width.max(min.width).min(max.width);
	let new_height = r.height.max(min.height).min(max.height);
	let diff_width = new_width.saturating_sub(r.width);
	let diff_height = new_height.saturating_sub(r.height);

	Rect::new(
		r.x.saturating_sub(diff_width / 2),
		r.y.saturating_sub(diff_height / 2),
		new_width,
		new_height,
	)
}

pub fn centered_rect_absolute(
	width: u16,
	height: u16,
	r: Rect,
) -> Rect {
	Rect::new(
		(r.width.saturating_sub(width)) / 2,
		(r.height.saturating_sub(height)) / 2,
		width.min(r.width),
		height.min(r.height),
	)
}

///
pub fn common_nav(
	key: crossterm::event::KeyEvent,
	key_config: &SharedKeyConfig,
) -> Option<MoveSelection> {
	if key == key_config.move_down {
		Some(MoveSelection::Down)
	} else if key == key_config.move_up {
		Some(MoveSelection::Up)
	} else if key == key_config.page_up {
		Some(MoveSelection::PageUp)
	} else if key == key_config.page_down {
		Some(MoveSelection::PageDown)
	} else if key == key_config.move_right {
		Some(MoveSelection::Right)
	} else if key == key_config.move_left {
		Some(MoveSelection::Left)
	} else if key == key_config.home || key == key_config.shift_up {
		Some(MoveSelection::Top)
	} else if key == key_config.end || key == key_config.shift_down {
		Some(MoveSelection::End)
	} else {
		None
	}
}
