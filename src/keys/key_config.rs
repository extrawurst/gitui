use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use std::{path::PathBuf, rc::Rc};

use crate::{args::get_app_config_path, strings::symbol};

use super::{
	key_list::{GituiKeyEvent, KeysList},
	symbols::KeySymbols,
};

pub type SharedKeyConfig = Rc<KeyConfig>;

#[derive(Default, Clone)]
pub struct KeyConfig {
	pub keys: KeysList,
	symbols: KeySymbols,
}

impl KeyConfig {
	fn get_config_file() -> Result<PathBuf> {
		let app_home = get_app_config_path()?;
		Ok(app_home.join("key_bindings.ron"))
	}

	fn get_symbols_file() -> Result<PathBuf> {
		let app_home = get_app_config_path()?;
		Ok(app_home.join("key_symbols.ron"))
	}

	pub fn init() -> Result<Self> {
		let keys = KeysList::init(Self::get_config_file()?);
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

	pub fn get_hint(&self, ev: GituiKeyEvent) -> String {
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
			_ => String::new(),
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
	use crossterm::event::{KeyCode, KeyModifiers};

	#[test]
	fn test_get_hint() {
		let config = KeyConfig::default();
		let h = config.get_hint(GituiKeyEvent::new(
			KeyCode::Char('c'),
			KeyModifiers::CONTROL,
		));
		assert_eq!(h, "^c");
	}
}
