use crossterm::event::Event;
use tui::{backend::Backend, layout::Rect, Frame};

mod command;
mod commit;
mod diff;
mod help;
mod index;
pub use command::CommandInfo;
pub use commit::CommitComponent;
pub use diff::DiffComponent;
pub use help::HelpComponent;
pub use index::IndexComponent;

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
    /// focus/unfocus this component depending on param
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
