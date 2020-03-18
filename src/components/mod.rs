use crossterm::event::Event;
use tui::{backend::Backend, layout::Rect, Frame};

mod commit;
mod index;
pub use commit::CommitComponent;
pub use index::IndexComponent;

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
    fn is_visible(&self) -> bool {
        true
    }
    ///
    fn hide(&mut self) {}
    ///
    fn show(&mut self) {}
}
