use crossterm::event::Event;
use tui::{backend::Backend, layout::Rect, Frame};

mod commit;
mod diff;
mod index;
pub use commit::CommitComponent;
pub use diff::DiffComponent;
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
    fn focused(&self) -> bool {
        false
    }
    ///
    fn focus(&mut self, _focus: bool) {}
    ///
    fn is_visible(&self) -> bool {
        true
    }
    ///
    fn hide(&mut self) {}
    ///
    fn show(&mut self) {}
}
