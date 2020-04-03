use std::{cell::RefCell, collections::VecDeque, rc::Rc};

///
pub enum InternalEvent {
    ///
    ConfirmResetFile(String),
    ///
    ResetFile(String),
}

///
pub type Queue = Rc<RefCell<VecDeque<InternalEvent>>>;
