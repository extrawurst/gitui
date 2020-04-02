use crossterm::event::Event;
use tui::{backend::Backend, layout::Rect, Frame};

mod changes;
mod command;
mod commit;
mod diff;
mod help;
pub use changes::ChangesComponent;
pub use command::CommandInfo;
pub use commit::CommitComponent;
pub use diff::DiffComponent;
pub use help::HelpComponent;

///
pub enum EventUpdate {
    None,
    All,
    Diff,
    Commands,
}

///
#[derive(PartialEq)]
pub enum CommandBlocking {
    Blocking,
    PassingOn,
}

///
pub fn visibility_blocking<T: Component>(
    comp: &T,
) -> CommandBlocking {
    if comp.is_visible() {
        CommandBlocking::Blocking
    } else {
        CommandBlocking::PassingOn
    }
}

pub trait DrawableComponent {
    ///
    fn draw<B: Backend>(&self, f: &mut Frame<B>, rect: Rect);
}
///
pub trait Component {
    ///
    fn commands(&self, out: &mut Vec<CommandInfo>)
        -> CommandBlocking;
    ///
    fn event(&mut self, ev: Event) -> Option<EventUpdate>;
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
