use crate::get_app_config_path;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ron::{
    de::from_bytes,
    ser::{to_string_pretty, PrettyConfig},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    rc::Rc,
};

pub type SharedKeyConfig = Rc<KeyConfig>;

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyConfig {
    pub tab_status: KeyEvent,
    pub tab_log: KeyEvent,
    pub tab_stashing: KeyEvent,
    pub tab_stashes: KeyEvent,
    pub tab_toggle: KeyEvent,
    pub tab_toggle_reverse: KeyEvent,
    pub focus_workdir: KeyEvent,
    pub focus_stage: KeyEvent,
    pub focus_right: KeyEvent,
    pub focus_left: KeyEvent,
    pub focus_above: KeyEvent,
    pub focus_below: KeyEvent,
    pub exit: KeyEvent,
    pub exit_popup: KeyEvent,
    pub open_commit: KeyEvent,
    pub open_commit_editor: KeyEvent,
    pub open_help: KeyEvent,
    pub move_left: KeyEvent,
    pub move_right: KeyEvent,
    pub home: KeyEvent,
    pub end: KeyEvent,
    pub move_up: KeyEvent,
    pub move_down: KeyEvent,
    pub page_down: KeyEvent,
    pub page_up: KeyEvent,
    pub shift_up: KeyEvent,
    pub shift_down: KeyEvent,
    pub enter: KeyEvent,
    pub edit_file: KeyEvent,
    pub status_stage_all: KeyEvent,
    pub status_reset_item: KeyEvent,
    pub status_ignore_file: KeyEvent,
    pub stashing_save: KeyEvent,
    pub stashing_toggle_untracked: KeyEvent,
    pub stashing_toggle_index: KeyEvent,
    pub stash_open: KeyEvent,
    pub stash_drop: KeyEvent,
    pub cmd_bar_toggle: KeyEvent,
    pub log_tag_commit: KeyEvent,
    pub commit_amend: KeyEvent,
    pub copy: KeyEvent,
    pub create_branch: KeyEvent,
    pub rename_branch: KeyEvent,
    pub select_branch: KeyEvent,
    pub delete_branch: KeyEvent,
    pub push: KeyEvent,
    pub fetch: KeyEvent,

    pub enter_symbol: String,
    pub left_symbol: String,
    pub right_symbol: String,
    pub up_symbol: String,
    pub down_symbol: String,
    pub backspace_symbol: String,
    pub home_symbol: String,
    pub end_symbol: String,
    pub page_up_symbol: String,
    pub page_down_symbol: String,
    pub tab_symbol: String,
    pub back_tab_symbol: String,
    pub delete_symbol: String,
    pub insert_symbol: String,
    pub esc_symbol: String,
    pub control_symbol: String,
    pub shift_symbol: String,
    pub alt_symbol: String,
}

#[rustfmt::skip]
impl Default for KeyConfig {
    fn default() -> Self {
        Self {
            tab_status: KeyEvent { code: KeyCode::Char('1'), modifiers: KeyModifiers::empty()},
			tab_log: KeyEvent { code: KeyCode::Char('2'), modifiers: KeyModifiers::empty()},
			tab_stashing: KeyEvent { code: KeyCode::Char('3'), modifiers: KeyModifiers::empty()},
			tab_stashes: KeyEvent { code: KeyCode::Char('4'), modifiers: KeyModifiers::empty()},
			tab_toggle: KeyEvent { code: KeyCode::Tab, modifiers: KeyModifiers::empty()},
			tab_toggle_reverse: KeyEvent { code: KeyCode::BackTab, modifiers: KeyModifiers::SHIFT},
			focus_workdir: KeyEvent { code: KeyCode::Char('w'), modifiers: KeyModifiers::empty()},
			focus_stage: KeyEvent { code: KeyCode::Char('s'), modifiers: KeyModifiers::empty()},
			focus_right: KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::empty()},
			focus_left: KeyEvent { code: KeyCode::Left, modifiers: KeyModifiers::empty()},
			focus_above: KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::empty()},
			focus_below: KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::empty()},
			exit: KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL},
			exit_popup: KeyEvent { code: KeyCode::Esc, modifiers: KeyModifiers::empty()},
			open_commit: KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::empty()},
			open_commit_editor: KeyEvent { code: KeyCode::Char('e'), modifiers:KeyModifiers::CONTROL},
			open_help: KeyEvent { code: KeyCode::Char('h'), modifiers: KeyModifiers::empty()},
			move_left: KeyEvent { code: KeyCode::Left, modifiers: KeyModifiers::empty()},
			move_right: KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::empty()},
			home: KeyEvent { code: KeyCode::Home, modifiers: KeyModifiers::empty()},
			end: KeyEvent { code: KeyCode::End, modifiers: KeyModifiers::empty()},
			move_up: KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::empty()},
			move_down: KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::empty()},
			page_down: KeyEvent { code: KeyCode::PageDown, modifiers: KeyModifiers::empty()},
			page_up: KeyEvent { code: KeyCode::PageUp, modifiers: KeyModifiers::empty()},
			shift_up: KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::SHIFT},
			shift_down: KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::SHIFT},
			enter: KeyEvent { code: KeyCode::Enter, modifiers: KeyModifiers::empty()},
			edit_file: KeyEvent { code: KeyCode::Char('e'), modifiers: KeyModifiers::empty()},
			status_stage_all: KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::empty()},
			status_reset_item: KeyEvent { code: KeyCode::Char('D'), modifiers: KeyModifiers::SHIFT},
			status_ignore_file: KeyEvent { code: KeyCode::Char('i'), modifiers: KeyModifiers::empty()},
			stashing_save: KeyEvent { code: KeyCode::Char('s'), modifiers: KeyModifiers::empty()},
			stashing_toggle_untracked: KeyEvent { code: KeyCode::Char('u'), modifiers: KeyModifiers::empty()},
			stashing_toggle_index: KeyEvent { code: KeyCode::Char('i'), modifiers: KeyModifiers::empty()},
			stash_open: KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::empty()},
			stash_drop: KeyEvent { code: KeyCode::Char('D'), modifiers: KeyModifiers::SHIFT},
			cmd_bar_toggle: KeyEvent { code: KeyCode::Char('.'), modifiers: KeyModifiers::empty()},
			log_tag_commit: KeyEvent { code: KeyCode::Char('t'), modifiers: KeyModifiers::empty()},
			commit_amend: KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::CONTROL},
            copy: KeyEvent { code: KeyCode::Char('y'), modifiers: KeyModifiers::empty()},
            create_branch: KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::NONE},
            rename_branch: KeyEvent { code: KeyCode::Char('r'), modifiers: KeyModifiers::NONE},
            select_branch: KeyEvent { code: KeyCode::Char('b'), modifiers: KeyModifiers::NONE},
            delete_branch: KeyEvent{code: KeyCode::Char('D'), modifiers: KeyModifiers::SHIFT},
            push: KeyEvent { code: KeyCode::Char('p'), modifiers: KeyModifiers::empty()},
            fetch: KeyEvent { code: KeyCode::Char('f'), modifiers: KeyModifiers::empty()},

            enter_symbol: "\u{23ce}".into(),     //⏎
            left_symbol: "\u{2190}".into(),      //←
            right_symbol: "\u{2192}".into(),     //→
            up_symbol: "\u{2191}".into(),        //↑
            down_symbol: "\u{2193}".into(),      //↓
            backspace_symbol: "\u{232b}".into(), //⌫
            home_symbol: "\u{2912}".into(),      //⤒
            end_symbol: "\u{2913}".into(),       //⤓
            page_up_symbol: "\u{21de}".into(),   //⇞
            page_down_symbol: "\u{21df}".into(), //⇟
            tab_symbol: "\u{21e5}".into(),       //⇥
            back_tab_symbol: "\u{21e4}".into(),  //⇤
            delete_symbol: "\u{2326}".into(),    //⌦
            insert_symbol: "\u{2380}".into(),    //⎀
            esc_symbol: "\u{238b}".into(),       //⎋
            control_symbol: "^".into(),
            shift_symbol: "\u{21e7}".into(),     //⇧
            alt_symbol: "\u{2325}".into(),       //⌥
        }
    }
}

impl KeyConfig {
    fn save(&self) -> Result<()> {
        let config_file = Self::get_config_file()?;
        let mut file = File::create(config_file)?;
        let data = to_string_pretty(self, PrettyConfig::default())?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }

    fn get_config_file() -> Result<PathBuf> {
        let app_home = get_app_config_path()?;
        Ok(app_home.join("key_config.ron"))
    }

    fn read_file(config_file: PathBuf) -> Result<Self> {
        let mut f = File::open(config_file)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        Ok(from_bytes(&buffer)?)
    }

    fn init_internal() -> Result<Self> {
        let file = Self::get_config_file()?;
        if file.exists() {
            Ok(Self::read_file(file)?)
        } else {
            let def = Self::default();
            if def.save().is_err() {
                log::warn!(
                    "failed to store default key config to disk."
                )
            }
            Ok(def)
        }
    }

    pub fn init() -> Self {
        match Self::init_internal() {
            Ok(v) => v,
            Err(e) => {
                log::error!("failed loading key binding: {}", e);
                Self::default()
            }
        }
    }

    fn get_key_symbol(&self, k: KeyCode) -> &str {
        match k {
            KeyCode::Enter => &self.enter_symbol,
            KeyCode::Left => &self.left_symbol,
            KeyCode::Right => &self.right_symbol,
            KeyCode::Up => &self.up_symbol,
            KeyCode::Down => &self.down_symbol,
            KeyCode::Backspace => &self.backspace_symbol,
            KeyCode::Home => &self.home_symbol,
            KeyCode::End => &self.end_symbol,
            KeyCode::PageUp => &self.page_up_symbol,
            KeyCode::PageDown => &self.page_down_symbol,
            KeyCode::Tab => &self.tab_symbol,
            KeyCode::BackTab => &self.back_tab_symbol,
            KeyCode::Delete => &self.delete_symbol,
            KeyCode::Insert => &self.insert_symbol,
            KeyCode::Esc => &self.esc_symbol,
            _ => "?",
        }
    }

    pub fn get_hint(&self, ev: KeyEvent) -> String {
        match ev.code {
            KeyCode::Down
            | KeyCode::Up
            | KeyCode::Right
            | KeyCode::Left
            | KeyCode::Enter
            | KeyCode::Backspace
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Tab
            | KeyCode::BackTab
            | KeyCode::Delete
            | KeyCode::Insert
            | KeyCode::Esc => {
                format!(
                    "{}{}",
                    self.get_modifier_hint(ev.modifiers),
                    self.get_key_symbol(ev.code)
                )
            }
            KeyCode::Char(c) => {
                format!(
                    "{}{}",
                    self.get_modifier_hint(ev.modifiers),
                    c
                )
            }
            KeyCode::F(u) => {
                format!(
                    "{}F{}",
                    self.get_modifier_hint(ev.modifiers),
                    u
                )
            }
            KeyCode::Null => self.get_modifier_hint(ev.modifiers).into(),
        }
    }

    fn get_modifier_hint(&self, modifier: KeyModifiers) -> &str {
        match modifier {
            KeyModifiers::CONTROL => &self.control_symbol,
            KeyModifiers::SHIFT => &self.shift_symbol,
            KeyModifiers::ALT => &self.alt_symbol,
            _ => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::KeyConfig;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_get_hint() {
        let config = KeyConfig::default();
        let h = config.get_hint(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        });
        assert_eq!(h, "^c");
    }

    #[test]
    fn test_load_vim_style_example() {
        assert_eq!(
            KeyConfig::read_file(
                "assets/vim_style_key_config.ron".into()
            )
            .is_ok(),
            true
        );
    }

    #[test]
    fn test_load_alternate_symbol_example() {
        assert_eq!(
            KeyConfig::read_file(
                "assets/alternate_key_symbols.ron".into()
            )
                .is_ok(),
            true
        )
    }
}
