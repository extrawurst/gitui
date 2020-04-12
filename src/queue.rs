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

///
pub enum InternalEvent {
    ///
    ConfirmResetFile(String),
    ///
    ResetFile(String),
    ///
    AddHunk(u64),
    ///
    ShowMsg(String),
    ///
    Update(NeedsUpdate),
}

///
pub type Queue = Rc<RefCell<VecDeque<InternalEvent>>>;
