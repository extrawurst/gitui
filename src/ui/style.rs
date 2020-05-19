use crate::get_app_config_path;
use asyncgit::{DiffLineType, StatusItemType};
use ron::de::from_bytes;
use ron::ser::{to_string_pretty, PrettyConfig};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use tui::style::{Color, Modifier, Style};

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy)]
pub struct Theme {
    selected_tab: ColorDef,
    command_foreground: ColorDef,
    command_background: ColorDef,
    command_disabled: ColorDef,
    diff_line_add: ColorDef,
    diff_line_delete: ColorDef,
    diff_file_added: ColorDef,
    diff_file_removed: ColorDef,
    diff_file_moved: ColorDef,
    diff_file_modified: ColorDef,
    table_colors: [ColorDef; 3],
}

pub const DARK_THEME: Theme = Theme {
    selected_tab: ColorDef::Yellow,
    command_foreground: ColorDef::White,
    command_background: ColorDef::Rgb(0, 0, 100),
    command_disabled: ColorDef::DarkGray,
    diff_line_add: ColorDef::Green,
    diff_line_delete: ColorDef::Red,
    diff_file_added: ColorDef::LightGreen,
    diff_file_removed: ColorDef::LightRed,
    diff_file_moved: ColorDef::LightMagenta,
    diff_file_modified: ColorDef::Yellow,
    table_colors: [
        ColorDef::Magenta,
        ColorDef::Blue,
        ColorDef::Green,
    ],
};

impl Theme {
    pub fn block(&self, focus: bool) -> Style {
        if focus {
            Style::default()
        } else {
            Style::default().fg(self.command_disabled.into())
        }
    }

    pub fn tab(&self, selected: bool) -> Style {
        if selected {
            Style::default().fg(self.selected_tab.into())
        } else {
            Style::default()
        }
    }

    pub fn text(&self, enabled: bool, selected: bool) -> Style {
        match (enabled, selected) {
            (false, _) => {
                Style::default().fg(self.command_disabled.into())
            }
            (true, false) => Style::default(),
            (true, true) => {
                Style::default().bg(self.command_background.into())
            }
        }
    }

    pub fn item(&self, typ: StatusItemType, selected: bool) -> Style {
        let style = match typ {
            StatusItemType::New => {
                Style::default().fg(self.diff_file_added.into())
            }
            StatusItemType::Modified => {
                Style::default().fg(self.diff_file_modified.into())
            }
            StatusItemType::Deleted => {
                Style::default().fg(self.diff_file_removed.into())
            }
            StatusItemType::Renamed => {
                Style::default().fg(self.diff_file_moved.into())
            }
            _ => Style::default(),
        };

        self.apply_select(style, selected)
    }

    fn apply_select(&self, style: Style, selected: bool) -> Style {
        if selected {
            style.bg(self.command_background.into())
        } else {
            style
        }
    }

    pub fn diff_line(
        &self,
        typ: DiffLineType,
        selected: bool,
    ) -> Style {
        let style = match typ {
            DiffLineType::Add => {
                Style::default().fg(self.diff_line_add.into())
            }
            DiffLineType::Delete => {
                Style::default().fg(self.diff_line_delete.into())
            }
            DiffLineType::Header => {
                Style::default().modifier(Modifier::BOLD)
            }
            _ => Style::default(),
        };

        self.apply_select(style, selected)
    }

    pub fn text_danger(&self) -> Style {
        Style::default().fg(self.diff_file_removed.into())
    }

    pub fn toolbar(&self, enabled: bool) -> Style {
        if enabled {
            Style::default().fg(self.command_foreground.into())
        } else {
            Style::default().fg(self.command_disabled.into())
        }
        .bg(self.command_background.into())
    }

    pub fn table(&self, column: usize, selected: bool) -> Style {
        self.apply_select(
            Style::default().fg(self.table_colors[column].into()),
            selected,
        )
    }

    fn save(&self) -> Result<(), std::io::Error> {
        let theme_file = Self::get_theme_file();
        let mut file = File::create(theme_file)?;
        let data = to_string_pretty(self, PrettyConfig::default())
            .map_err(|_| std::io::Error::from_raw_os_error(100))?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }

    fn get_theme_file() -> PathBuf {
        let app_home = get_app_config_path();
        app_home.join("theme.ron")
    }

    fn read_file(
        theme_file: PathBuf,
    ) -> Result<Theme, std::io::Error> {
        if theme_file.exists() {
            let mut f = File::open(theme_file)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;

            Ok(from_bytes(&buffer).map_err(|_| {
                std::io::Error::from_raw_os_error(100)
            })?)
        } else {
            Err(std::io::Error::from_raw_os_error(100))
        }
    }

    pub fn init() -> Theme {
        if let Ok(x) = Theme::read_file(Theme::get_theme_file()) {
            x
        } else {
            DARK_THEME.save().unwrap_or_default();
            DARK_THEME
        }
    }
}

/// we duplicate the Color definition from `tui` crate to implement Serde serialisation
/// this enum can be removed once [tui-#292](https://github.com/fdehau/tui-rs/issues/292) is resolved
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum ColorDef {
    Reset,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    White,
    Rgb(u8, u8, u8),
    Indexed(u8),
}

impl Default for ColorDef {
    fn default() -> Self {
        ColorDef::Reset
    }
}

impl From<ColorDef> for Color {
    fn from(def: ColorDef) -> Self {
        match def {
            ColorDef::Reset => Color::Reset,
            ColorDef::Black => Color::Black,
            ColorDef::Red => Color::Red,
            ColorDef::Green => Color::Green,
            ColorDef::Yellow => Color::Yellow,
            ColorDef::Blue => Color::Blue,
            ColorDef::Magenta => Color::Magenta,
            ColorDef::Cyan => Color::Cyan,
            ColorDef::Gray => Color::Gray,
            ColorDef::DarkGray => Color::DarkGray,
            ColorDef::LightRed => Color::LightRed,
            ColorDef::LightGreen => Color::LightGreen,
            ColorDef::LightYellow => Color::LightYellow,
            ColorDef::LightBlue => Color::LightBlue,
            ColorDef::LightMagenta => Color::LightMagenta,
            ColorDef::LightCyan => Color::LightCyan,
            ColorDef::White => Color::White,
            ColorDef::Rgb(a, b, c) => Color::Rgb(a, b, c),
            ColorDef::Indexed(x) => Color::Indexed(x),
        }
    }
}
