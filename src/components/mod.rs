mod changes;
mod command;
mod commit;
mod diff;
mod filetree;
mod help;
mod msg;
mod reset;
mod stashmsg;
mod textinput;
mod utils;
use anyhow::Result;
pub use changes::ChangesComponent;
pub use command::{CommandInfo, CommandText};
pub use commit::CommitComponent;
use crossterm::event::Event;
pub use diff::DiffComponent;
pub use filetree::FileTreeComponent;
pub use help::HelpComponent;
pub use msg::MsgComponent;
pub use reset::ResetComponent;
pub use stashmsg::StashMsgComponent;
pub use utils::filetree::FileTreeItemKind;

use tui::{
    backend::Backend,
    layout::Alignment,
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Text},
    Frame,
};

/// creates accessors for a list of components
///
/// allows generating code to make sure
/// we always enumerate all components in both getter functions
#[macro_export]
macro_rules! accessors {
    ($self:ident, [$($element:ident),+]) => {
        fn components(& $self) -> Vec<&dyn Component> {
            vec![
                $(&$self.$element,)+
            ]
        }

        fn components_mut(&mut $self) -> Vec<&mut dyn Component> {
            vec![
                $(&mut $self.$element,)+
            ]
        }
    };
}

/// returns `true` if event was consumed
pub fn event_pump(
    ev: Event,
    components: &mut [&mut dyn Component],
) -> Result<bool> {
    for c in components {
        if c.event(ev)? {
            return Ok(true);
        }
    }

    Ok(false)
}

/// helper fn to simplify delegating command
/// gathering down into child components
/// see `event_pump`,`accessors`
pub fn command_pump(
    out: &mut Vec<CommandInfo>,
    force_all: bool,
    components: &[&dyn Component],
) {
    for c in components {
        if c.commands(out, force_all) != CommandBlocking::PassingOn
            && !force_all
        {
            break;
        }
    }
}

#[derive(Copy, Clone)]
pub enum ScrollType {
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
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
    fn draw<B: Backend>(
        &mut self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()>;
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
    fn event(&mut self, ev: Event) -> Result<bool>;

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
    fn show(&mut self) -> Result<()> {
        Ok(())
    }
}

fn dialog_paragraph<'a, 't, T>(
    title: &'a str,
    content: T,
) -> Paragraph<'a, 't, T>
where
    T: Iterator<Item = &'t Text<'t>>,
{
    Paragraph::new(content)
        .block(Block::default().title(title).borders(Borders::ALL))
        .alignment(Alignment::Left)
}
