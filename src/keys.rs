use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const fn no_mod(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
    }
}

pub static FOCUS_STATUS: KeyEvent = no_mod(KeyCode::Char('1'));
pub static FOCUS_DIFF: KeyEvent = no_mod(KeyCode::Char('2'));
pub static FOCUS_STAGE: KeyEvent = no_mod(KeyCode::Char('3'));
