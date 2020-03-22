pub mod diff;
pub mod status;
pub mod utils;

pub use status::get_index;
pub use utils::{commit, index_reset, stage_add, stage_reset};
