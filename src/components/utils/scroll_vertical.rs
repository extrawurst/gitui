use std::cell::Cell;

use tui::{backend::Backend, layout::Rect, Frame};

use crate::{
    components::ScrollType,
    ui::{draw_scrollbar, style::SharedTheme},
};

pub struct VerticalScroll {
    top: Cell<usize>,
    max_top: Cell<usize>,
}

impl VerticalScroll {
    pub const fn new() -> Self {
        Self {
            top: Cell::new(0),
            max_top: Cell::new(0),
        }
    }

    pub fn get_top(&self) -> usize {
        self.top.get()
    }

    pub fn reset(&self) {
        self.top.set(0);
    }

    pub fn move_top(&self, move_type: ScrollType) -> bool {
        let old = self.top.get();
        let max = self.max_top.get();

        let new_scroll_top = match move_type {
            ScrollType::Down => old.saturating_add(1),
            ScrollType::Up => old.saturating_sub(1),
            ScrollType::Home => 0,
            ScrollType::End => max,
            _ => old,
        };

        let new_scroll_top = new_scroll_top.clamp(0, max);

        if new_scroll_top == old {
            return false;
        }

        self.top.set(new_scroll_top);

        true
    }

    pub fn update(
        &self,
        selection: usize,
        selection_max: usize,
        visual_height: usize,
    ) -> usize {
        let new_top = calc_scroll_top(
            self.get_top(),
            visual_height,
            selection,
            selection_max,
        );
        self.top.set(new_top);

        let new_max = selection_max.saturating_sub(visual_height);
        self.max_top.set(new_max);

        new_top
    }

    pub fn update_no_selection(
        &self,
        line_count: usize,
        visual_height: usize,
    ) -> usize {
        self.update(self.get_top(), line_count, visual_height)
    }

    pub fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        r: Rect,
        theme: &SharedTheme,
    ) {
        draw_scrollbar(
            f,
            r,
            theme,
            self.max_top.get(),
            self.top.get(),
        );
    }
}

const fn calc_scroll_top(
    current_top: usize,
    height_in_lines: usize,
    selection: usize,
    selection_max: usize,
) -> usize {
    if selection_max <= height_in_lines {
        return 0;
    }

    if current_top + height_in_lines <= selection {
        selection.saturating_sub(height_in_lines) + 1
    } else if current_top > selection {
        selection
    } else {
        current_top
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_scroll_no_scroll_to_top() {
        assert_eq!(calc_scroll_top(1, 10, 4, 4), 0);
    }
}
