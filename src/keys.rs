//TODO: remove once fixed https://github.com/rust-lang/rust-clippy/issues/6818
#![allow(clippy::use_self)]

use anyhow::{Error, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ron::{
    self,
    ser::{to_string_pretty, PrettyConfig},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
    rc::Rc,
};

use crate::args::get_app_config_path;

pub type SharedKeyConfig = Rc<KeyConfig>;

trait Save {
    fn save(&self, path: PathBuf) -> Result<()>;
}

fn handle_error_and_default<T: Save + Default>(
    e: &Error,
    file: PathBuf,
) -> Result<T> {
    let config_path = file.clone();
    let config_path_old = format!("{}.old", file.to_string_lossy());
    fs::rename(config_path.clone(), config_path_old.clone())?;

    T::default().save(file)?;

    Err(anyhow::anyhow!("{}\n Old file was renamed to {:?}.\n Defaults loaded and saved as {:?}",
                        e,config_path_old,config_path.to_string_lossy()))
}

fn read_file<T>(config_file: PathBuf) -> Result<T>
where
    T: DeserializeOwned,
{
    let mut f = File::open(config_file)?;
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)?;
    Ok(ron::de::from_bytes(&buffer)?)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SymbolConfig {
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
impl Default for SymbolConfig {
    fn default() -> Self {
        Self {
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

impl Save for SymbolConfig {
    fn save(&self, file: PathBuf) -> Result<()> {
        let mut file = File::create(file)?;
        let data = to_string_pretty(self, PrettyConfig::default())?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }
}

impl SymbolConfig {
    pub fn get_config_file() -> Result<PathBuf> {
        let app_home = get_app_config_path()?;
        Ok(app_home.join("symbol_config.ron"))
    }

    fn init(file: PathBuf) -> Result<Self> {
        if file.exists() {
            match read_file(file.clone()) {
                Err(e) => handle_error_and_default(&e, file),
                Ok(res) => Ok(res),
            }
        } else {
            Self::default().save(file)?;
            Ok(Self::default())
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyConfig {
    pub tab_status: KeyEvent,
    pub tab_log: KeyEvent,
    pub tab_files: KeyEvent,
    pub tab_stashing: KeyEvent,
    pub tab_stashes: KeyEvent,
    pub tab_toggle: KeyEvent,
    pub tab_toggle_reverse: KeyEvent,
    pub toggle_workarea: KeyEvent,
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
    pub tree_collapse_recursive: KeyEvent,
    pub tree_expand_recursive: KeyEvent,
    pub home: KeyEvent,
    pub end: KeyEvent,
    pub move_up: KeyEvent,
    pub move_down: KeyEvent,
    pub page_down: KeyEvent,
    pub page_up: KeyEvent,
    pub shift_up: KeyEvent,
    pub shift_down: KeyEvent,
    pub enter: KeyEvent,
    pub blame: KeyEvent,
    pub edit_file: KeyEvent,
    pub status_stage_all: KeyEvent,
    pub status_reset_item: KeyEvent,
    pub status_ignore_file: KeyEvent,
    pub diff_stage_lines: KeyEvent,
    pub diff_reset_lines: KeyEvent,
    pub stashing_save: KeyEvent,
    pub stashing_toggle_untracked: KeyEvent,
    pub stashing_toggle_index: KeyEvent,
    pub stash_apply: KeyEvent,
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
    pub merge_branch: KeyEvent,
    pub tags: KeyEvent,
    pub delete_tag: KeyEvent,
    pub select_tag: KeyEvent,
    pub push: KeyEvent,
    pub open_file_tree: KeyEvent,
    pub force_push: KeyEvent,
    pub pull: KeyEvent,
    pub abort_merge: KeyEvent,

    #[serde(skip)]
    symbols: SymbolConfig,
}

#[rustfmt::skip]
impl Default for KeyConfig {
    fn default() -> Self {
        Self {
			tab_status: KeyEvent { code: KeyCode::Char('1'), modifiers: KeyModifiers::empty()},
			tab_log: KeyEvent { code: KeyCode::Char('2'), modifiers: KeyModifiers::empty()},
            tab_files: KeyEvent { code: KeyCode::Char('3'), modifiers: KeyModifiers::empty()},
			tab_stashing: KeyEvent { code: KeyCode::Char('4'), modifiers: KeyModifiers::empty()},
			tab_stashes: KeyEvent { code: KeyCode::Char('5'), modifiers: KeyModifiers::empty()},
			tab_toggle: KeyEvent { code: KeyCode::Tab, modifiers: KeyModifiers::empty()},
			tab_toggle_reverse: KeyEvent { code: KeyCode::BackTab, modifiers: KeyModifiers::SHIFT},
            toggle_workarea: KeyEvent { code: KeyCode::Char('w'), modifiers: KeyModifiers::empty()},
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
            tree_collapse_recursive: KeyEvent { code: KeyCode::Left, modifiers: KeyModifiers::SHIFT},
            tree_expand_recursive: KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::SHIFT},
			home: KeyEvent { code: KeyCode::Home, modifiers: KeyModifiers::empty()},
			end: KeyEvent { code: KeyCode::End, modifiers: KeyModifiers::empty()},
			move_up: KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::empty()},
			move_down: KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::empty()},
			page_down: KeyEvent { code: KeyCode::PageDown, modifiers: KeyModifiers::empty()},
			page_up: KeyEvent { code: KeyCode::PageUp, modifiers: KeyModifiers::empty()},
			shift_up: KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::SHIFT},
			shift_down: KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::SHIFT},
			enter: KeyEvent { code: KeyCode::Enter, modifiers: KeyModifiers::empty()},
			blame: KeyEvent { code: KeyCode::Char('B'), modifiers: KeyModifiers::SHIFT},
			edit_file: KeyEvent { code: KeyCode::Char('e'), modifiers: KeyModifiers::empty()},
			status_stage_all: KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::empty()},
			status_reset_item: KeyEvent { code: KeyCode::Char('D'), modifiers: KeyModifiers::SHIFT},
            diff_reset_lines: KeyEvent { code: KeyCode::Char('d'), modifiers: KeyModifiers::empty()},
			status_ignore_file: KeyEvent { code: KeyCode::Char('i'), modifiers: KeyModifiers::empty()},
            diff_stage_lines: KeyEvent { code: KeyCode::Char('s'), modifiers: KeyModifiers::empty()},
			stashing_save: KeyEvent { code: KeyCode::Char('s'), modifiers: KeyModifiers::empty()},
			stashing_toggle_untracked: KeyEvent { code: KeyCode::Char('u'), modifiers: KeyModifiers::empty()},
			stashing_toggle_index: KeyEvent { code: KeyCode::Char('i'), modifiers: KeyModifiers::empty()},
			stash_apply: KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::empty()},
			stash_open: KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::empty()},
			stash_drop: KeyEvent { code: KeyCode::Char('D'), modifiers: KeyModifiers::SHIFT},
			cmd_bar_toggle: KeyEvent { code: KeyCode::Char('.'), modifiers: KeyModifiers::empty()},
			log_tag_commit: KeyEvent { code: KeyCode::Char('t'), modifiers: KeyModifiers::empty()},
			commit_amend: KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::CONTROL},
            copy: KeyEvent { code: KeyCode::Char('y'), modifiers: KeyModifiers::empty()},
            create_branch: KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::empty()},
            rename_branch: KeyEvent { code: KeyCode::Char('r'), modifiers: KeyModifiers::empty()},
            select_branch: KeyEvent { code: KeyCode::Char('b'), modifiers: KeyModifiers::empty()},
            delete_branch: KeyEvent { code: KeyCode::Char('D'), modifiers: KeyModifiers::SHIFT},
            merge_branch: KeyEvent { code: KeyCode::Char('m'), modifiers: KeyModifiers::empty()},
            tags: KeyEvent { code: KeyCode::Char('T'), modifiers: KeyModifiers::SHIFT},
            delete_tag: KeyEvent { code: KeyCode::Char('D'), modifiers: KeyModifiers::SHIFT},
            select_tag: KeyEvent { code: KeyCode::Enter, modifiers: KeyModifiers::empty()},
            push: KeyEvent { code: KeyCode::Char('p'), modifiers: KeyModifiers::empty()},
            force_push: KeyEvent { code: KeyCode::Char('P'), modifiers: KeyModifiers::SHIFT},
            pull: KeyEvent { code: KeyCode::Char('f'), modifiers: KeyModifiers::empty()},
            abort_merge: KeyEvent { code: KeyCode::Char('M'), modifiers: KeyModifiers::SHIFT},
            open_file_tree: KeyEvent { code: KeyCode::Char('F'), modifiers: KeyModifiers::SHIFT},

            symbols: SymbolConfig::default(),
        }
    }
}

impl Save for KeyConfig {
    fn save(&self, file: PathBuf) -> Result<()> {
        let mut file = File::create(file)?;
        let data = to_string_pretty(self, PrettyConfig::default())?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }
}

impl KeyConfig {
    pub fn get_config_file() -> Result<PathBuf> {
        let app_home = get_app_config_path()?;
        Ok(app_home.join("key_config.ron"))
    }

    pub fn init(
        key_config_file: PathBuf,
        symbol_config_file: PathBuf,
    ) -> Result<Self> {
        let mut key_config_file =
            Self::init_key_config(key_config_file)?;
        key_config_file.symbols =
            SymbolConfig::init(symbol_config_file)?;
        Ok(key_config_file)
    }

    fn init_key_config(file: PathBuf) -> Result<Self> {
        if file.exists() {
            match read_file(file.clone()) {
                Err(e) => handle_error_and_default(&e, file),
                Ok(res) => Ok(res),
            }
        } else {
            Self::default().save(file)?;
            Ok(Self::default())
        }
    }

    fn get_key_symbol(&self, k: KeyCode) -> &str {
        match k {
            KeyCode::Enter => &self.symbols.enter_symbol,
            KeyCode::Left => &self.symbols.left_symbol,
            KeyCode::Right => &self.symbols.right_symbol,
            KeyCode::Up => &self.symbols.up_symbol,
            KeyCode::Down => &self.symbols.down_symbol,
            KeyCode::Backspace => &self.symbols.backspace_symbol,
            KeyCode::Home => &self.symbols.home_symbol,
            KeyCode::End => &self.symbols.end_symbol,
            KeyCode::PageUp => &self.symbols.page_up_symbol,
            KeyCode::PageDown => &self.symbols.page_down_symbol,
            KeyCode::Tab => &self.symbols.tab_symbol,
            KeyCode::BackTab => &self.symbols.back_tab_symbol,
            KeyCode::Delete => &self.symbols.delete_symbol,
            KeyCode::Insert => &self.symbols.insert_symbol,
            KeyCode::Esc => &self.symbols.esc_symbol,
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
            KeyCode::Null => {
                self.get_modifier_hint(ev.modifiers).into()
            }
        }
    }

    fn get_modifier_hint(&self, modifier: KeyModifiers) -> &str {
        match modifier {
            KeyModifiers::CONTROL => &self.symbols.control_symbol,
            KeyModifiers::SHIFT => &self.symbols.shift_symbol,
            KeyModifiers::ALT => &self.symbols.alt_symbol,
            _ => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::KeyConfig;
    use crate::keys::{read_file, SymbolConfig};
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
            read_file::<KeyConfig>("vim_style_key_config.ron".into())
                .is_ok(),
            true
        );
    }

    #[test]
    fn test_load_alternate_symbol_example() {
        assert_eq!(
            read_file::<SymbolConfig>(
                "assets/alternate_key_symbols.ron".into()
            )
            .is_ok(),
            true
        )
    }
}
