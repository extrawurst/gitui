use std::borrow::Cow;
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph, Text},
    Frame,
};

#[derive(Default)]
pub struct Revlog {
    test: String,
}

impl Revlog {
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let txt = vec![Text::Raw(Cow::from(self.test.as_str()))];
        f.render_widget(
            Paragraph::new(txt.iter())
                .block(
                    Block::default()
                        .title("log")
                        .borders(Borders::ALL),
                )
                .alignment(Alignment::Left),
            area,
        );
    }
}
