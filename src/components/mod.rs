use crossterm::event::Event;
use tui::{backend::Backend, layout::Rect, Frame};

mod changes;
mod command;
mod commit;
mod diff;
mod filetree;
mod help;
mod msg;
mod reset;
mod statustree;
pub use changes::ChangesComponent;
pub use command::{CommandInfo, CommandText};
pub use commit::CommitComponent;
pub use diff::DiffComponent;
pub use filetree::FileTreeItemKind;
pub use help::HelpComponent;
pub use msg::MsgComponent;
pub use reset::ResetComponent;

#[derive(Copy, Clone)]
pub enum ScrollType {
    Up,
    Down,
    Home,
    End,
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

///
pub trait DrawableComponent {
    ///
    fn draw<B: Backend>(&self, f: &mut Frame<B>, rect: Rect);
}

/// base component trait
pub trait Component {
    ///
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking;

    /// returns true if event propagation needs to end (event was consumed)
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
