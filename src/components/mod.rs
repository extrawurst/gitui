mod blame_file;
mod branch_find_popup;
mod branchlist;
mod changes;
mod command;
mod commit;
mod commit_details;
mod commitlist;
mod compare_commits;
mod create_branch;
mod cred;
mod diff;
mod externaleditor;
mod fetch;
mod file_find_popup;
mod file_revlog;
mod help;
mod inspect_commit;
mod msg;
mod options_popup;
mod pull;
mod push;
mod push_tags;
mod rename_branch;
mod reset;
mod reset_popup;
mod revision_files;
mod revision_files_popup;
mod stashmsg;
mod status_tree;
mod submodules;
mod syntax_text;
mod tag_commit;
mod taglist;
mod textinput;
mod utils;

pub use self::status_tree::StatusTreeComponent;
pub use blame_file::{BlameFileComponent, BlameFileOpen};
pub use branch_find_popup::BranchFindPopup;
pub use branchlist::BranchListComponent;
pub use changes::ChangesComponent;
pub use command::{CommandInfo, CommandText};
pub use commit::CommitComponent;
pub use commit_details::CommitDetailsComponent;
pub use commitlist::CommitList;
pub use compare_commits::CompareCommitsComponent;
pub use create_branch::CreateBranchComponent;
pub use diff::DiffComponent;
pub use externaleditor::ExternalEditorComponent;
pub use fetch::FetchComponent;
pub use file_find_popup::FileFindPopup;
pub use file_revlog::{FileRevOpen, FileRevlogComponent};
pub use help::HelpComponent;
pub use inspect_commit::{InspectCommitComponent, InspectCommitOpen};
pub use msg::MsgComponent;
pub use options_popup::{AppOption, OptionsPopupComponent};
pub use pull::PullComponent;
pub use push::PushComponent;
pub use push_tags::PushTagsComponent;
pub use rename_branch::RenameBranchComponent;
pub use reset::ConfirmComponent;
pub use reset_popup::ResetPopupComponent;
pub use revision_files::RevisionFilesComponent;
pub use revision_files_popup::{FileTreeOpen, RevisionFilesPopup};
pub use stashmsg::StashMsgComponent;
pub use submodules::SubmodulesListComponent;
pub use syntax_text::SyntaxTextComponent;
pub use tag_commit::TagCommitComponent;
pub use taglist::TagListComponent;
pub use textinput::{InputType, TextInputComponent};
pub use utils::filetree::FileTreeItemKind;

use crate::ui::style::Theme;
use anyhow::Result;
use crossterm::event::Event;
use ratatui::{
	backend::Backend,
	layout::{Alignment, Rect},
	text::{Span, Text},
	widgets::{Block, BorderType, Borders, Paragraph, Wrap},
	Frame,
};
use std::convert::From;

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

/// creates a function to determine if any popup is visible
#[macro_export]
macro_rules! any_popup_visible {
    ($self:ident, [$($element:ident),+]) => {
        fn any_popup_visible(& $self) -> bool{
            ($($self.$element.is_visible()) || +)
        }
    };
}

/// creates the draw popup function
#[macro_export]
macro_rules! draw_popups {
    ($self:ident, [$($element:ident),+]) => {
        fn draw_popups<B: Backend>(& $self, mut f: &mut Frame<B>) -> Result<()>{
            //TODO: move the layout part out and feed it into `draw_popups`
            let size = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Min(1),
                    Constraint::Length($self.cmdbar.borrow().height()),
                ]
                .as_ref(),
            )
            .split(f.size())[0];

            ($($self.$element.draw(&mut f, size)?) , +);

            return Ok(());
        }
    };
}

/// simply calls
/// `any_popup_visible`!() and `draw_popups`!() macros
#[macro_export]
macro_rules! setup_popups {
    ($self:ident, [$($element:ident),+]) => {
        $crate::any_popup_visible!($self, [$($element),+]);
        $crate::draw_popups!($self, [ $($element),+ ]);
    };
}

/// returns `true` if event was consumed
pub fn event_pump(
	ev: &Event,
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
pub enum HorizontalScrollType {
	Left,
	Right,
}

#[derive(Copy, Clone)]
pub enum Direction {
	Up,
	Down,
}

///
#[derive(PartialEq, Eq)]
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
#[derive(PartialEq, Eq)]
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
	fn event(&mut self, ev: &Event) -> Result<EventState>;

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

fn popup_paragraph<'a, T>(
	title: &'a str,
	content: T,
	theme: &Theme,
	focused: bool,
	block: bool,
) -> Paragraph<'a>
where
	T: Into<Text<'a>>,
{
	let paragraph = Paragraph::new(content.into())
		.alignment(Alignment::Left)
		.wrap(Wrap { trim: true });

	if block {
		paragraph.block(
			Block::default()
				.title(Span::styled(title, theme.title(focused)))
				.borders(Borders::ALL)
				.border_type(BorderType::Thick)
				.border_style(theme.block(focused)),
		)
	} else {
		paragraph
	}
}
