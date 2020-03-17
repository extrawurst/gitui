use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::widgets::Widget;

pub struct Clear<T: Widget>(T);

impl<T: Widget> Clear<T> {
    pub fn new(w: T) -> Self {
        Self(w)
    }
}

impl<T: Widget> Widget for Clear<T> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        for x in area.left()..area.right() {
            for y in area.top()..area.bottom() {
                buf.get_mut(x, y).reset();
            }
        }

        self.0.draw(area, buf);
    }
}
