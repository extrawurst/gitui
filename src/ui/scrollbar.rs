use super::style::SharedTheme;
use std::convert::TryFrom;
use tui::{
    backend::Backend,
    buffer::Buffer,
    layout::{Margin, Rect},
    style::Style,
    symbols::{block::FULL, line::DOUBLE_VERTICAL},
    widgets::Widget,
    Frame,
};

///
struct Scrollbar {
    lines: u16,
    pos: u16,
    style_bar: Style,
    style_pos: Style,
}

impl Scrollbar {
    fn new(lines: usize, pos: usize) -> Self {
        Self {
            lines: u16::try_from(lines).unwrap_or_default(),
            pos: u16::try_from(pos).unwrap_or_default(),
            style_pos: Style::default(),
            style_bar: Style::default(),
        }
    }
}

impl Widget for Scrollbar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let right = area.right().saturating_sub(1);
        if right <= area.left() {
            return;
        };

        let area = area.inner(&Margin {
            horizontal: 0,
            vertical: 1,
        });

        if area.height == 0 {
            return;
        }

        if area.height >= self.lines {
            return;
        }

        for y in area.top()..area.bottom() {
            buf.set_string(right, y, DOUBLE_VERTICAL, self.style_bar);
        }

        let max_pos = self.lines.saturating_sub(area.height);
        let progress = f32::from(self.pos) / f32::from(max_pos);
        let progress = if progress > 1.0 { 1.0 } else { progress };
        let pos = f32::from(area.height) * progress;

        //TODO: any better way for this?
        #[allow(clippy::cast_sign_loss)]
        #[allow(clippy::cast_possible_truncation)]
        let pos = (pos as u16).saturating_sub(1);

        buf.set_string(right, area.top() + pos, FULL, self.style_pos);
    }
}

pub fn draw_scrollbar<B: Backend>(
    f: &mut Frame<B>,
    r: Rect,
    theme: &SharedTheme,
    lines: usize,
    pos: usize,
) {
    let mut widget = Scrollbar::new(lines, pos);
    widget.style_pos = theme.scroll_bar_pos();
    f.render_widget(widget, r)
}
