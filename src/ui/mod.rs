mod scrolllist;

use scrolllist::ScrollableList;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Text},
    Frame,
};

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

pub fn draw_list<'b, B: Backend, L>(
    f: &mut Frame<B>,
    r: Rect,
    title: &'b str,
    items: L,
    select: Option<usize>,
    selected: bool,
) where
    L: Iterator<Item = Text<'b>>,
{
    let mut style_border = Style::default().fg(Color::DarkGray);
    let mut style_title = Style::default();
    if selected {
        style_border = style_border.fg(Color::Gray);
        style_title = style_title.modifier(Modifier::BOLD);
    }
    let list = ScrollableList::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .title_style(style_title)
                .border_style(style_border),
        )
        .scroll(select.unwrap_or_default())
        .style(Style::default().fg(Color::White));
    f.render_widget(list, r)
}
