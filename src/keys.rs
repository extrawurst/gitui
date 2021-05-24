//TODO: remove once fixed https://github.com/rust-lang/rust-clippy/issues/6818
#![allow(clippy::use_self)]

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
    rc::Rc,
};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ron::{
    self,
    ser::{PrettyConfig, to_string_pretty},
};
use serde::{Deserialize, Serialize};

use crate::args::get_app_config_path;

pub type SharedKeyConfig = Rc<KeyConfig>;

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyConfig {
    pub tab_status: KeyEvent,
    #[serde(default = "KeyConfig::default_tab_log")]
    pub tab_log: KeyEvent,
    #[serde(default = "KeyConfig::default_tab_stashing")]
    pub tab_stashing: KeyEvent,
    #[serde(default = "KeyConfig::default_tab_stashes")]
    pub tab_stashes: KeyEvent,
    #[serde(default = "KeyConfig::default_tab_toggle")]
    pub tab_toggle: KeyEvent,
    #[serde(default = "KeyConfig::default_tab_toggle_reverse")]
    pub tab_toggle_reverse: KeyEvent,
    #[serde(default = "KeyConfig::default_toggle_workarea")]
    pub toggle_workarea: KeyEvent,
    #[serde(default = "KeyConfig::default_focus_right")]
    pub focus_right: KeyEvent,
    #[serde(default = "KeyConfig::default_focus_left")]
    pub focus_left: KeyEvent,
    #[serde(default = "KeyConfig::default_focus_above")]
    pub focus_above: KeyEvent,
    #[serde(default = "KeyConfig::default_focus_below")]
    pub focus_below: KeyEvent,
    #[serde(default = "KeyConfig::default_exit")]
    pub exit: KeyEvent,
    #[serde(default = "KeyConfig::default_exit_popup")]
    pub exit_popup: KeyEvent,
    #[serde(default = "KeyConfig::default_open_commit")]
    pub open_commit: KeyEvent,
    #[serde(default = "KeyConfig::default_open_commit_editor")]
    pub open_commit_editor: KeyEvent,
    #[serde(default = "KeyConfig::default_open_help")]
    pub open_help: KeyEvent,
    #[serde(default = "KeyConfig::default_move_left")]
    pub move_left: KeyEvent,
    #[serde(default = "KeyConfig::default_move_right")]
    pub move_right: KeyEvent,
    #[serde(default = "KeyConfig::default_tree_collapse_recursive")]
    pub tree_collapse_recursive: KeyEvent,
    #[serde(default = "KeyConfig::default_tree_expand_recursive")]
    pub tree_expand_recursive: KeyEvent,
    #[serde(default = "KeyConfig::default_home")]
    pub home: KeyEvent,
    #[serde(default = "KeyConfig::default_end")]
    pub end: KeyEvent,
    #[serde(default = "KeyConfig::default_move_up")]
    pub move_up: KeyEvent,
    #[serde(default = "KeyConfig::default_move_down")]
    pub move_down: KeyEvent,
    #[serde(default = "KeyConfig::default_page_down")]
    pub page_down: KeyEvent,
    #[serde(default = "KeyConfig::default_page_up")]
    pub page_up: KeyEvent,
    #[serde(default = "KeyConfig::default_shift_up")]
    pub shift_up: KeyEvent,
    #[serde(default = "KeyConfig::default_shift_down")]
    pub shift_down: KeyEvent,
    #[serde(default = "KeyConfig::default_enter")]
    pub enter: KeyEvent,
    #[serde(default = "KeyConfig::default_blame")]
    pub blame: KeyEvent,
    #[serde(default = "KeyConfig::default_edit_file")]
    pub edit_file: KeyEvent,
    #[serde(default = "KeyConfig::default_status_stage_all")]
    pub status_stage_all: KeyEvent,
    #[serde(default = "KeyConfig::default_status_reset_item")]
    pub status_reset_item: KeyEvent,
    #[serde(default = "KeyConfig::default_status_ignore_file")]
    pub status_ignore_file: KeyEvent,
    #[serde(default = "KeyConfig::default_diff_stage_lines")]
    pub diff_stage_lines: KeyEvent,
    #[serde(default = "KeyConfig::default_diff_reset_lines")]
    pub diff_reset_lines: KeyEvent,
    #[serde(default = "KeyConfig::default_stashing_save")]
    pub stashing_save: KeyEvent,
    #[serde(default = "KeyConfig::default_stashing_toggle_untracked")]
    pub stashing_toggle_untracked: KeyEvent,
    #[serde(default = "KeyConfig::default_stashing_toggle_index")]
    pub stashing_toggle_index: KeyEvent,
    #[serde(default = "KeyConfig::default_stash_apply")]
    pub stash_apply: KeyEvent,
    #[serde(default = "KeyConfig::default_stash_open")]
    pub stash_open: KeyEvent,
    #[serde(default = "KeyConfig::default_stash_drop")]
    pub stash_drop: KeyEvent,
    #[serde(default = "KeyConfig::default_cmd_bar_toggle")]
    pub cmd_bar_toggle: KeyEvent,
    #[serde(default = "KeyConfig::default_log_tag_commit")]
    pub log_tag_commit: KeyEvent,
    #[serde(default = "KeyConfig::default_commit_amend")]
    pub commit_amend: KeyEvent,
    #[serde(default = "KeyConfig::default_copy")]
    pub copy: KeyEvent,
    #[serde(default = "KeyConfig::default_create_branch")]
    pub create_branch: KeyEvent,
    #[serde(default = "KeyConfig::default_rename_branch")]
    pub rename_branch: KeyEvent,
    #[serde(default = "KeyConfig::default_select_branch")]
    pub select_branch: KeyEvent,
    #[serde(default = "KeyConfig::default_delete_branch")]
    pub delete_branch: KeyEvent,
    #[serde(default = "KeyConfig::default_merge_branch")]
    pub merge_branch: KeyEvent,
    #[serde(default = "KeyConfig::default_push")]
    pub push: KeyEvent,
    #[serde(default = "KeyConfig::default_open_file_tree")]
    pub open_file_tree: KeyEvent,
    #[serde(default = "KeyConfig::default_force_push")]
    pub force_push: KeyEvent,
    #[serde(default = "KeyConfig::default_pull")]
    pub pull: KeyEvent,
    #[serde(default = "KeyConfig::default_abort_merge")]
    pub abort_merge: KeyEvent,
}

impl KeyConfig {
    fn default_tab_status() -> KeyEvent { KeyEvent { code: KeyCode::Char('1'), modifiers: KeyModifiers::empty() } }
    fn default_tab_log() -> KeyEvent { KeyEvent { code: KeyCode::Char('2'), modifiers: KeyModifiers::empty() } }
    fn default_tab_stashing() -> KeyEvent { KeyEvent { code: KeyCode::Char('3'), modifiers: KeyModifiers::empty() } }
    fn default_tab_stashes() -> KeyEvent { KeyEvent { code: KeyCode::Char('4'), modifiers: KeyModifiers::empty() } }
    fn default_tab_toggle() -> KeyEvent { KeyEvent { code: KeyCode::Tab, modifiers: KeyModifiers::empty() } }
    fn default_tab_toggle_reverse() -> KeyEvent { KeyEvent { code: KeyCode::BackTab, modifiers: KeyModifiers::SHIFT } }
    fn default_toggle_workarea() -> KeyEvent { KeyEvent { code: KeyCode::Char('w'), modifiers: KeyModifiers::empty() } }
    fn default_focus_right() -> KeyEvent { KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::empty() } }
    fn default_focus_left() -> KeyEvent { KeyEvent { code: KeyCode::Left, modifiers: KeyModifiers::empty() } }
    fn default_focus_above() -> KeyEvent { KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::empty() } }
    fn default_focus_below() -> KeyEvent { KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::empty() } }
    fn default_exit() -> KeyEvent { KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL } }
    fn default_exit_popup() -> KeyEvent { KeyEvent { code: KeyCode::Esc, modifiers: KeyModifiers::empty() } }
    fn default_open_commit() -> KeyEvent { KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::empty() } }
    fn default_open_commit_editor() -> KeyEvent { KeyEvent { code: KeyCode::Char('e'), modifiers: KeyModifiers::CONTROL } }
    fn default_open_help() -> KeyEvent { KeyEvent { code: KeyCode::Char('h'), modifiers: KeyModifiers::empty() } }
    fn default_move_left() -> KeyEvent { KeyEvent { code: KeyCode::Left, modifiers: KeyModifiers::empty() } }
    fn default_move_right() -> KeyEvent { KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::empty() } }
    fn default_tree_collapse_recursive() -> KeyEvent { KeyEvent { code: KeyCode::Left, modifiers: KeyModifiers::SHIFT } }
    fn default_tree_expand_recursive() -> KeyEvent { KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::SHIFT } }
    fn default_home() -> KeyEvent { KeyEvent { code: KeyCode::Home, modifiers: KeyModifiers::empty() } }
    fn default_end() -> KeyEvent { KeyEvent { code: KeyCode::End, modifiers: KeyModifiers::empty() } }
    fn default_move_up() -> KeyEvent { KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::empty() } }
    fn default_move_down() -> KeyEvent { KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::empty() } }
    fn default_page_down() -> KeyEvent { KeyEvent { code: KeyCode::PageDown, modifiers: KeyModifiers::empty() } }
    fn default_page_up() -> KeyEvent { KeyEvent { code: KeyCode::PageUp, modifiers: KeyModifiers::empty() } }
    fn default_shift_up() -> KeyEvent { KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::SHIFT } }
    fn default_shift_down() -> KeyEvent { KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::SHIFT } }
    fn default_enter() -> KeyEvent { KeyEvent { code: KeyCode::Enter, modifiers: KeyModifiers::empty() } }
    fn default_blame() -> KeyEvent { KeyEvent { code: KeyCode::Char('B'), modifiers: KeyModifiers::SHIFT } }
    fn default_edit_file() -> KeyEvent { KeyEvent { code: KeyCode::Char('e'), modifiers: KeyModifiers::empty() } }
    fn default_status_stage_all() -> KeyEvent { KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::empty() } }
    fn default_status_reset_item() -> KeyEvent { KeyEvent { code: KeyCode::Char('D'), modifiers: KeyModifiers::SHIFT } }
    fn default_diff_reset_lines() -> KeyEvent { KeyEvent { code: KeyCode::Char('d'), modifiers: KeyModifiers::empty() } }
    fn default_status_ignore_file() -> KeyEvent { KeyEvent { code: KeyCode::Char('i'), modifiers: KeyModifiers::empty() } }
    fn default_diff_stage_lines() -> KeyEvent { KeyEvent { code: KeyCode::Char('s'), modifiers: KeyModifiers::empty() } }
    fn default_stashing_save() -> KeyEvent { KeyEvent { code: KeyCode::Char('s'), modifiers: KeyModifiers::empty() } }
    fn default_stashing_toggle_untracked() -> KeyEvent { KeyEvent { code: KeyCode::Char('u'), modifiers: KeyModifiers::empty() } }
    fn default_stashing_toggle_index() -> KeyEvent { KeyEvent { code: KeyCode::Char('i'), modifiers: KeyModifiers::empty() } }
    fn default_stash_apply() -> KeyEvent { KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::empty() } }
    fn default_stash_open() -> KeyEvent { KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::empty() } }
    fn default_stash_drop() -> KeyEvent { KeyEvent { code: KeyCode::Char('D'), modifiers: KeyModifiers::SHIFT } }
    fn default_cmd_bar_toggle() -> KeyEvent { KeyEvent { code: KeyCode::Char('.'), modifiers: KeyModifiers::empty() } }
    fn default_log_tag_commit() -> KeyEvent { KeyEvent { code: KeyCode::Char('t'), modifiers: KeyModifiers::empty() } }
    fn default_commit_amend() -> KeyEvent { KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::CONTROL } }
    fn default_copy() -> KeyEvent { KeyEvent { code: KeyCode::Char('y'), modifiers: KeyModifiers::empty() } }
    fn default_create_branch() -> KeyEvent { KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::empty() } }
    fn default_rename_branch() -> KeyEvent { KeyEvent { code: KeyCode::Char('r'), modifiers: KeyModifiers::empty() } }
    fn default_select_branch() -> KeyEvent { KeyEvent { code: KeyCode::Char('b'), modifiers: KeyModifiers::empty() } }
    fn default_delete_branch() -> KeyEvent { KeyEvent { code: KeyCode::Char('D'), modifiers: KeyModifiers::SHIFT } }
    fn default_merge_branch() -> KeyEvent { KeyEvent { code: KeyCode::Char('m'), modifiers: KeyModifiers::empty() } }
    fn default_push() -> KeyEvent { KeyEvent { code: KeyCode::Char('p'), modifiers: KeyModifiers::empty() } }
    fn default_force_push() -> KeyEvent { KeyEvent { code: KeyCode::Char('P'), modifiers: KeyModifiers::SHIFT } }
    fn default_pull() -> KeyEvent { KeyEvent { code: KeyCode::Char('f'), modifiers: KeyModifiers::empty() } }
    fn default_abort_merge() -> KeyEvent { KeyEvent { code: KeyCode::Char('M'), modifiers: KeyModifiers::SHIFT } }
    fn default_open_file_tree() -> KeyEvent { KeyEvent { code: KeyCode::Char('F'), modifiers: KeyModifiers::SHIFT } }
}

#[rustfmt::skip]
impl Default for KeyConfig {
    fn default() -> Self {
        Self {
            tab_status: Self::default_tab_status(),
            tab_log: Self::default_tab_log(),
            tab_stashing: Self::default_tab_stashing(),
            tab_stashes: Self::default_tab_stashes(),
            tab_toggle: Self::default_tab_toggle(),
            tab_toggle_reverse: Self::default_tab_toggle_reverse(),
            toggle_workarea: Self::default_toggle_workarea(),
            focus_right: Self::default_focus_right(),
            focus_left: Self::default_focus_left(),
            focus_above: Self::default_focus_above(),
            focus_below: Self::default_focus_below(),
            exit: Self::default_exit(),
            exit_popup: Self::default_exit_popup(),
            open_commit: Self::default_open_commit(),
            open_commit_editor: Self::default_open_commit_editor(),
            open_help: Self::default_open_help(),
            move_left: Self::default_move_left(),
            move_right: Self::default_move_right(),
            tree_collapse_recursive: Self::default_tree_collapse_recursive(),
            tree_expand_recursive: Self::default_tree_expand_recursive(),
            home: Self::default_home(),
            end: Self::default_end(),
            move_up: Self::default_move_up(),
            move_down: Self::default_move_down(),
            page_down: Self::default_page_down(),
            page_up: Self::default_page_up(),
            shift_up: Self::default_shift_up(),
            shift_down: Self::default_shift_down(),
            enter: Self::default_enter(),
            blame: Self::default_blame(),
            edit_file: Self::default_edit_file(),
            status_stage_all: Self::default_status_stage_all(),
            status_reset_item: Self::default_status_reset_item(),
            diff_reset_lines: Self::default_diff_reset_lines(),
            status_ignore_file: Self::default_status_ignore_file(),
            diff_stage_lines: Self::default_diff_stage_lines(),
            stashing_save: Self::default_stashing_save(),
            stashing_toggle_untracked: Self::default_stashing_toggle_untracked(),
            stashing_toggle_index: Self::default_stashing_toggle_index(),
            stash_apply: Self::default_stash_apply(),
            stash_open: Self::default_stash_open(),
            stash_drop: Self::default_stash_drop(),
            cmd_bar_toggle: Self::default_cmd_bar_toggle(),
            log_tag_commit: Self::default_log_tag_commit(),
            commit_amend: Self::default_commit_amend(),
            copy: Self::default_copy(),
            create_branch: Self::default_create_branch(),
            rename_branch: Self::default_rename_branch(),
            select_branch: Self::default_select_branch(),
            delete_branch: Self::default_delete_branch(),
            merge_branch: Self::default_merge_branch(),
            push: Self::default_push(),
            force_push: Self::default_force_push(),
            pull: Self::default_pull(),
            abort_merge: Self::default_abort_merge(),
            open_file_tree: Self::default_open_file_tree(),
        }
    }
}

impl KeyConfig {
    fn save(&self, file: PathBuf) -> Result<()> {
        let mut file = File::create(file)?;
        let data = to_string_pretty(self, PrettyConfig::default())?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }

    pub fn get_config_file() -> Result<PathBuf> {
        let app_home = get_app_config_path()?;
        Ok(app_home.join("key_config.ron"))
    }

    fn read_file(config_file: PathBuf) -> Result<Self> {
        let mut f = File::open(config_file)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        Ok(ron::de::from_bytes(&buffer)?)
    }

    pub fn init(file: PathBuf) -> Result<Self> {
        if file.exists() {
            match Self::read_file(file.clone()) {
                Err(e) => {
                    let config_path = file.clone();
                    let config_path_old =
                        format!("{}.old", file.to_string_lossy());
                    fs::rename(
                        config_path.clone(),
                        config_path_old.clone(),
                    )?;

                    Self::default().save(file)?;

                    Err(anyhow::anyhow!("{}\n Old file was renamed to {:?}.\n Defaults loaded and saved as {:?}",
                        e,config_path_old,config_path.to_string_lossy()))
                }
                Ok(res) => Ok(res),
            }
        } else {
            Self::default().save(file)?;
            Ok(Self::default())
        }
    }

    //TODO: make this configurable (https://github.com/extrawurst/gitui/issues/465)
    #[allow(clippy::unused_self)]
    const fn get_key_symbol(&self, k: KeyCode) -> &str {
        match k {
            KeyCode::Enter => "\u{23ce}",     //⏎
            KeyCode::Left => "\u{2190}",      //←
            KeyCode::Right => "\u{2192}",     //→
            KeyCode::Up => "\u{2191}",        //↑
            KeyCode::Down => "\u{2193}",      //↓
            KeyCode::Backspace => "\u{232b}", //⌫
            KeyCode::Home => "\u{2912}",      //⤒
            KeyCode::End => "\u{2913}",       //⤓
            KeyCode::PageUp => "\u{21de}",    //⇞
            KeyCode::PageDown => "\u{21df}",  //⇟
            KeyCode::Tab => "\u{21e5}",       //⇥
            KeyCode::BackTab => "\u{21e4}",   //⇤
            KeyCode::Delete => "\u{2326}",    //⌦
            KeyCode::Insert => "\u{2380}",    //⎀
            KeyCode::Esc => "\u{238b}",       //⎋
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
                    Self::get_modifier_hint(ev.modifiers),
                    self.get_key_symbol(ev.code)
                )
            }
            KeyCode::Char(c) => {
                format!(
                    "{}{}",
                    Self::get_modifier_hint(ev.modifiers),
                    c
                )
            }
            KeyCode::F(u) => {
                format!(
                    "{}F{}",
                    Self::get_modifier_hint(ev.modifiers),
                    u
                )
            }
            KeyCode::Null => Self::get_modifier_hint(ev.modifiers),
        }
    }

    //TODO: make customizable (see https://github.com/extrawurst/gitui/issues/465)
    fn get_modifier_hint(modifier: KeyModifiers) -> String {
        match modifier {
            KeyModifiers::CONTROL => "^".to_string(),
            KeyModifiers::SHIFT => {
                "\u{21e7}".to_string() //⇧
            }
            KeyModifiers::ALT => {
                "\u{2325}".to_string() //⌥
            }
            _ => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::KeyConfig;

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
            KeyConfig::read_file("vim_style_key_config.ron".into())
                .is_ok(),
            true
        );
    }

    #[test]
    fn test_key_config_fields_use_default_values_if_missing_during_deserialization() {
        let input = "( tab_status: ( code: Char('6'), modifiers: ( bits: 1, ), ), )";
        let result: ron::Result<KeyConfig> = ron::de::from_str(input);

        assert_eq!(result.is_ok(), true);
        let config = result.unwrap();

        let expected_tab_status = KeyEvent { code: KeyCode::Char('6'), modifiers: KeyModifiers::SHIFT };
        assert_eq!(config.tab_status, expected_tab_status);

        assert_eq!(config.tab_log, KeyConfig::default_tab_log());
    }
}
