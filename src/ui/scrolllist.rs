use std::iter::Iterator;
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, List, Text, Widget},
};

///
pub struct ScrollableList<'b, L>
where
    L: Iterator<Item = Text<'b>>,
{
    block: Option<Block<'b>>,
    /// Items to be displayed
    items: L,
    /// Index of the scroll position
    scroll: usize,
    /// Base style of the widget
    style: Style,
}

impl<'b, L> ScrollableList<'b, L>
where
    L: Iterator<Item = Text<'b>>,
{
    pub fn new(items: L) -> Self {
        Self {
            block: None,
            items,
            scroll: 0,
            style: Style::default(),
        }
    }

    pub fn block(mut self, block: Block<'b>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn scroll(mut self, index: usize) -> Self {
        self.scroll = index;
        self
    }
}

impl<'b, L> Widget for ScrollableList<'b, L>
where
    L: Iterator<Item = Text<'b>>,
{
    fn render(self, area: Rect, buf: &mut Buffer) {
        let list_area = match self.block {
            Some(b) => b.inner(area),
            None => area,
        };

        let list_height = list_area.height as usize;

        let offset = if self.scroll >= list_height {
            self.scroll - list_height + 1
        } else {
            0
        };

        // Render items
        List::new(self.items.skip(offset as usize))
            .block(self.block.unwrap_or_default())
            .style(self.style)
            .render(area, buf);
    }
}
