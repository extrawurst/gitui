//TODO: remove once fixed https://github.com/rust-lang/rust-clippy/issues/6818
#![allow(clippy::use_self)]

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{fs, path::PathBuf, rc::Rc};

use crate::{args::get_app_config_path, keys::Keys, strings::symbol};

pub type SharedKeyConfig = Rc<KeyConfig>;

#[derive(Default)]
pub struct KeyConfig {
	pub keys: Keys,
}

impl KeyConfig {
	pub fn get_config_file() -> Result<PathBuf> {
		let app_home = get_app_config_path()?;
		Ok(app_home.join("key_config.ron"))
	}

	pub fn init(file: PathBuf) -> Result<Self> {
		if file.exists() {
			match Keys::read_file(file.clone()) {
				Err(e) => {
					let config_path = file.clone();
					let config_path_old =
						format!("{}.old", file.to_string_lossy());
					fs::rename(
						config_path.clone(),
						config_path_old.clone(),
					)?;

					Keys::default().save(file)?;

					Err(anyhow::anyhow!("{}\n Old file was renamed to {:?}.\n Defaults loaded and saved as {:?}",
						e,config_path_old,config_path.to_string_lossy()))
				}
				Ok(keys) => Ok(Self { keys }),
			}
		} else {
			Keys::default().save(file)?;
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
			KeyCode::Char(' ') => String::from(symbol::SPACE),
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
	use super::*;
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
}
