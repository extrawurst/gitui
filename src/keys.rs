use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const fn no_mod(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
    }
}

pub static FOCUS_STATUS: KeyEvent = no_mod(KeyCode::Char('1'));
pub static FOCUS_STAGE: KeyEvent = no_mod(KeyCode::Char('2'));
pub static FOCUS_RIGHT: KeyEvent = no_mod(KeyCode::Right);
pub static FOCUS_LEFT: KeyEvent = no_mod(KeyCode::Left);
