use std::convert::TryFrom;
use tui::{
    backend::Backend,
    buffer::Buffer,
    layout::{Margin, Rect},
    style::{Color, Style},
    symbols::{block::FULL, line::THICK_VERTICAL},
    widgets::Widget,
    Frame,
};

///
struct Scrollbar {
    max: u16,
    pos: u16,
}

impl Scrollbar {
    fn new(max: usize, pos: usize) -> Self {
        Self {
            max: u16::try_from(max).unwrap_or_default(),
            pos: u16::try_from(pos).unwrap_or_default(),
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

        if area.height <= 4 {
            return;
        }

        if area.height > self.max {
            return;
        }

        let style = Style::default();
        for y in area.top()..area.bottom() {
            buf.set_string(right, y, THICK_VERTICAL, style);
        }

        let progress = f32::from(self.pos) / f32::from(self.max);
        let pos = f32::from(area.height.saturating_sub(1)) * progress;
        #[allow(clippy::cast_sign_loss)]
        #[allow(clippy::cast_possible_truncation)]
        let pos = pos as u16;

        buf.set_string(
            right,
            area.top() + pos,
            FULL,
            style.fg(Color::Blue),
        );
    }
}

pub fn draw_scrollbar<B: Backend>(
    f: &mut Frame<B>,
    r: Rect,
    max: usize,
    pos: usize,
) {
    let widget = Scrollbar::new(max, pos);
    f.render_widget(widget, r)
}
