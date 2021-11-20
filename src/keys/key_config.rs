//TODO: remove once fixed https://github.com/rust-lang/rust-clippy/issues/6818
#![allow(clippy::use_self)]

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{fs, path::PathBuf, rc::Rc};

use crate::{args::get_app_config_path, strings::symbol};

use super::{key_list::KeysList, symbols::KeySymbols};

pub type SharedKeyConfig = Rc<KeyConfig>;

#[derive(Default)]
pub struct KeyConfig {
	pub keys: KeysList,
	symbols: KeySymbols,
}

impl KeyConfig {
	fn get_config_file() -> Result<PathBuf> {
		let app_home = get_app_config_path()?;
		Ok(app_home.join("key_config.ron"))
	}

	fn get_symbols_file() -> Result<PathBuf> {
		let app_home = get_app_config_path()?;
		Ok(app_home.join("key_symbols.ron"))
	}

	fn init_keys() -> Result<KeysList> {
		let file = Self::get_config_file()?;
		if file.exists() {
			match KeysList::read_file(file.clone()) {
				Err(e) => {
					let config_path = file.clone();
					let config_path_old =
						format!("{}.old", file.to_string_lossy());
					fs::rename(
						config_path.clone(),
						config_path_old.clone(),
					)?;

					KeysList::default().save(file)?;

					Err(anyhow::anyhow!("{}\n Old file was renamed to {:?}.\n Defaults loaded and saved as {:?}",
						e,config_path_old,config_path.to_string_lossy()))
				}
				Ok(keys) => Ok(keys),
			}
		} else {
			KeysList::default().save(file)?;
			Ok(KeysList::default())
		}
	}

	pub fn init() -> Result<Self> {
		let keys = Self::init_keys()?;
		let symbols = KeySymbols::init(Self::get_symbols_file()?);
		Ok(Self { keys, symbols })
	}

	fn get_key_symbol(&self, k: KeyCode) -> &str {
		match k {
			KeyCode::Enter => &self.symbols.enter,
			KeyCode::Left => &self.symbols.left,
			KeyCode::Right => &self.symbols.right,
			KeyCode::Up => &self.symbols.up,
			KeyCode::Down => &self.symbols.down,
			KeyCode::Backspace => &self.symbols.backspace,
			KeyCode::Home => &self.symbols.home,
			KeyCode::End => &self.symbols.end,
			KeyCode::PageUp => &self.symbols.page_up,
			KeyCode::PageDown => &self.symbols.page_down,
			KeyCode::Tab => &self.symbols.tab,
			KeyCode::BackTab => &self.symbols.back_tab,
			KeyCode::Delete => &self.symbols.delete,
			KeyCode::Insert => &self.symbols.insert,
			KeyCode::Esc => &self.symbols.esc,
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
			KeyCode::Char(' ') => String::from(symbol::SPACE),
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
			KeyModifiers::CONTROL => &self.symbols.control,
			KeyModifiers::SHIFT => &self.symbols.shift,
			KeyModifiers::ALT => &self.symbols.alt,
			_ => "",
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
