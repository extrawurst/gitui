use crossterm::event::Event;
use tui::{backend::Backend, layout::Rect, Frame};

pub mod commit;

///
pub struct CommandInfo {
    pub name: String,
    pub enabled: bool,
}

///
pub trait Component {
    ///
    fn draw<B: Backend>(&self, f: &mut Frame<B>, rect: Rect);
    ///
    fn commands(&self) -> Vec<CommandInfo>;
    ///
    fn event(&mut self, ev: Event) -> bool;
    ///
    fn is_visible(&self) -> bool;
    ///
    fn hide(&mut self);
    ///
    fn show(&mut self);
}
