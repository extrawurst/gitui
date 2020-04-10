use std::{cell::RefCell, collections::VecDeque, rc::Rc};

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
}

///
pub type Queue = Rc<RefCell<VecDeque<InternalEvent>>>;
