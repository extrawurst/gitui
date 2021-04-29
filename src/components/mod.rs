mod blame_file;
mod branchlist;
mod changes;
mod command;
mod commit;
mod commit_details;
mod commitlist;
mod create_branch;
mod cred;
mod diff;
mod externaleditor;
mod filetree;
mod help;
mod inspect_commit;
mod msg;
mod pull;
mod push;
mod push_tags;
mod rename_branch;
mod reset;
mod stashmsg;
mod tag_commit;
mod textinput;
mod utils;

pub use blame_file::BlameFileComponent;
pub use branchlist::BranchListComponent;
pub use changes::ChangesComponent;
pub use command::{CommandInfo, CommandText};
pub use commit::CommitComponent;
pub use commit_details::CommitDetailsComponent;
pub use commitlist::CommitList;
pub use create_branch::CreateBranchComponent;
pub use diff::DiffComponent;
pub use externaleditor::ExternalEditorComponent;
pub use filetree::FileTreeComponent;
pub use help::HelpComponent;
pub use inspect_commit::InspectCommitComponent;
pub use msg::MsgComponent;
pub use pull::PullComponent;
pub use push::PushComponent;
pub use push_tags::PushTagsComponent;
pub use rename_branch::RenameBranchComponent;
pub use reset::ResetComponent;
pub use stashmsg::StashMsgComponent;
pub use tag_commit::TagCommitComponent;
pub use textinput::{InputType, TextInputComponent};
pub use utils::filetree::FileTreeItemKind;

use crate::ui::style::Theme;
use anyhow::Result;
use crossterm::event::Event;
use std::convert::From;
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    text::{Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
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
) -> Result<EventState> {
    for c in components {
        if c.event(ev)?.is_consumed() {
            return Ok(EventState::Consumed);
        }
    }

    Ok(EventState::NotConsumed)
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

#[derive(Copy, Clone)]
pub enum Direction {
    Up,
    Down,
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
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()>;
}

///
#[derive(PartialEq)]
pub enum EventState {
    Consumed,
    NotConsumed,
}

impl EventState {
    pub fn is_consumed(&self) -> bool {
        *self == Self::Consumed
    }
}

impl From<bool> for EventState {
    fn from(consumed: bool) -> Self {
        if consumed {
            Self::Consumed
        } else {
            Self::NotConsumed
        }
    }
}

/// base component trait
pub trait Component {
    ///
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking;

    ///
    fn event(&mut self, ev: Event) -> Result<EventState>;

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

    ///
    fn toggle_visible(&mut self) -> Result<()> {
        if self.is_visible() {
            self.hide();
            Ok(())
        } else {
            self.show()
        }
    }
}

fn dialog_paragraph<'a>(
    title: &'a str,
    content: Text<'a>,
    theme: &Theme,
    focused: bool,
) -> Paragraph<'a> {
    Paragraph::new(content)
        .block(
            Block::default()
                .title(Span::styled(title, theme.title(focused)))
                .borders(Borders::ALL)
                .border_style(theme.block(focused)),
        )
        .alignment(Alignment::Left)
}

fn popup_paragraph_commit<'a, T>(
    title: &'a str,
    content: T,
    theme: &Theme,
    focused: bool,
    commit_first_line_length: usize,
) -> Paragraph<'a>
where
    T: Into<Text<'a>>,
{
    println!("{}", commit_first_line_length);

    let text = content.into();

    let mut border_style = theme.block(focused);

    if commit_first_line_length > 50 {
        //border_style = theme.text_danger().patch(border_style);
    }

    popup_paragraph_inner(
        title,
        text,
        theme,
        focused,
        Some(border_style),
    )
}

fn popup_paragraph<'a, T>(
    title: &'a str,
    content: T,
    theme: &Theme,
    focused: bool,
) -> Paragraph<'a>
where
    T: Into<Text<'a>>,
{
    popup_paragraph_inner(title, content, theme, focused, None)
}

/// Use `popup_paragraph` or `popup_paragraph_commit` depending on need, they call this
fn popup_paragraph_inner<'a, T>(
    title: &'a str,
    content: T,
    theme: &Theme,
    focused: bool,
    border_style: Option<tui::style::Style>,
) -> Paragraph<'a>
where
    T: Into<Text<'a>>,
{
    Paragraph::new(content.into())
        .block(
            Block::default()
                .title(Span::styled(title, theme.title(focused)))
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .border_style(
                    border_style
                        .unwrap_or_else(|| theme.block(focused)),
                ),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true })
}
