use crate::tabs::StashingOptions;
use asyncgit::sync::{CommitId, CommitTags};
use bitflags::bitflags;
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

bitflags! {
    /// flags defining what part of the app need to update
    pub struct NeedsUpdate: u32 {
        /// app::update
        const ALL = 0b001;
        /// diff may have changed (app::update_diff)
        const DIFF = 0b010;
        /// commands might need updating (app::update_commands)
        const COMMANDS = 0b100;
    }
}

/// data of item that is supposed to be reset
pub struct ResetItem {
    /// path to the item (folder/file)
    pub path: String,
    /// are talking about a folder here? otherwise it's a single file
    pub is_folder: bool,
}

///
pub enum Action {
    Reset(ResetItem),
    ResetHunk(String, u64),
    StashDrop(CommitId),
    DeleteBranch(String),
    ForcePush(String, bool),
}

///
pub enum InternalEvent {
    ///
    ConfirmAction(Action),
    ///
    ConfirmedAction(Action),
    ///
    ShowErrorMsg(String),
    ///
    Update(NeedsUpdate),
    /// open commit msg input
    OpenCommit,
    ///
    PopupStashing(StashingOptions),
    ///
    TabSwitch,
    ///
    InspectCommit(CommitId, Option<CommitTags>),
    ///
    TagCommit(CommitId),
    ///
    CreateBranch,
    ///
    RenameBranch(String, String),
    ///
    SelectBranch,
    ///
    OpenExternalEditor(Option<String>),
    ///
    Push(String, bool),
    ///
    Fetch(String),
}

///
pub type Queue = Rc<RefCell<VecDeque<InternalEvent>>>;
