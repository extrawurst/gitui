use super::style::SharedTheme;
use std::iter::Iterator;
use tui::{
    backend::Backend,
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Borders, List, ListItem, Widget},
    Frame,
};

///
struct ScrollableList<'b, L>
where
    L: Iterator<Item = Span<'b>>,
{
    block: Option<Block<'b>>,
    /// Items to be displayed
    items: L,
    /// Base style of the widget
    style: Style,
}

impl<'b, L> ScrollableList<'b, L>
where
    L: Iterator<Item = Span<'b>>,
{
    fn new(items: L) -> Self {
        Self {
            block: None,
            items,
            style: Style::default(),
        }
    }

    fn block(mut self, block: Block<'b>) -> Self {
        self.block = Some(block);
        self
    }
}

impl<'b, L> Widget for ScrollableList<'b, L>
where
    L: Iterator<Item = Span<'b>>,
{
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Render items
        List::new(
            self.items.map(ListItem::new).collect::<Vec<ListItem>>(),
        )
        .block(self.block.unwrap_or_default())
        .style(self.style)
        .render(area, buf);
    }
}

pub fn draw_list<'b, B: Backend, L>(
    f: &mut Frame<B>,
    r: Rect,
    title: &'b str,
    items: L,
    selected: bool,
    theme: &SharedTheme,
) where
    L: Iterator<Item = Span<'b>>,
{
    let list = ScrollableList::new(items).block(
        Block::default()
            .title(Span::styled(title, theme.title(selected)))
            .borders(Borders::ALL)
            .border_style(theme.block(selected)),
    );
    f.render_widget(list, r);
}

pub fn draw_list_block<'b, B: Backend, L>(
    f: &mut Frame<B>,
    r: Rect,
    block: Block<'b>,
    items: L,
) where
    L: Iterator<Item = Span<'b>>,
{
    let list = ScrollableList::new(items).block(block);
    f.render_widget(list, r);
}
