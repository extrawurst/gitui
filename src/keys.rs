use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const fn no_mod(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
    }
}

const fn with_mod(
    code: KeyCode,
    modifiers: KeyModifiers,
) -> KeyEvent {
    KeyEvent { code, modifiers }
}

pub const TAB_TOGGLE: KeyEvent = no_mod(KeyCode::Tab);
pub const FOCUS_WORKDIR: KeyEvent = no_mod(KeyCode::Char('1'));
pub const FOCUS_STAGE: KeyEvent = no_mod(KeyCode::Char('2'));
pub const FOCUS_RIGHT: KeyEvent = no_mod(KeyCode::Right);
pub const FOCUS_LEFT: KeyEvent = no_mod(KeyCode::Left);
pub const EXIT: KeyEvent =
    with_mod(KeyCode::Char('c'), KeyModifiers::CONTROL);
pub const EXIT_POPUP: KeyEvent = no_mod(KeyCode::Esc);
pub const CLOSE_MSG: KeyEvent = no_mod(KeyCode::Enter);
pub const OPEN_COMMIT: KeyEvent = no_mod(KeyCode::Char('c'));
pub const OPEN_HELP: KeyEvent = no_mod(KeyCode::Char('h'));
pub const MOVE_LEFT: KeyEvent = no_mod(KeyCode::Left);
pub const MOVE_RIGHT: KeyEvent = no_mod(KeyCode::Right);
pub const HOME: KeyEvent = no_mod(KeyCode::Home);
pub const END: KeyEvent = no_mod(KeyCode::End);
pub const MOVE_UP: KeyEvent = no_mod(KeyCode::Up);
pub const MOVE_DOWN: KeyEvent = no_mod(KeyCode::Down);
pub const PAGE_DOWN: KeyEvent = no_mod(KeyCode::PageDown);
pub const PAGE_UP: KeyEvent = no_mod(KeyCode::PageUp);
pub const SHIFT_UP: KeyEvent =
    with_mod(KeyCode::Up, KeyModifiers::SHIFT);
pub const SHIFT_DOWN: KeyEvent =
    with_mod(KeyCode::Down, KeyModifiers::SHIFT);
pub const ENTER: KeyEvent = no_mod(KeyCode::Enter);
pub const STATUS_STAGE_FILE: KeyEvent = no_mod(KeyCode::Enter);
pub const STATUS_RESET_FILE: KeyEvent =
    with_mod(KeyCode::Char('D'), KeyModifiers::SHIFT);
