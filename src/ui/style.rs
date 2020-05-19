use crate::get_app_config_path;
use asyncgit::{DiffLineType, StatusItemType};
use ron::{
    de::from_bytes,
    ser::{to_string_pretty, PrettyConfig},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{
    fs::File,
    io::{Read, Write},
};
use tui::style::{Color, Modifier, Style};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Theme {
    #[serde(with = "ColorDef")]
    selected_tab: Color,
    #[serde(with = "ColorDef")]
    command_fg: Color,
    #[serde(with = "ColorDef")]
    selection_bg: Color,
    #[serde(with = "ColorDef")]
    disabled_fg: Color,
    #[serde(with = "ColorDef")]
    diff_line_add: Color,
    #[serde(with = "ColorDef")]
    diff_line_delete: Color,
    #[serde(with = "ColorDef")]
    diff_file_added: Color,
    #[serde(with = "ColorDef")]
    diff_file_removed: Color,
    #[serde(with = "ColorDef")]
    diff_file_moved: Color,
    #[serde(with = "ColorDef")]
    diff_file_modified: Color,
    #[serde(with = "ColorDef")]
    commit_hash: Color,
    #[serde(with = "ColorDef")]
    commit_time: Color,
    #[serde(with = "ColorDef")]
    commit_author: Color,
}

impl Theme {
    pub fn block(&self, focus: bool) -> Style {
        if focus {
            Style::default()
        } else {
            Style::default().fg(self.disabled_fg)
        }
    }

    pub fn title(&self, focused: bool) -> Style {
        if focused {
            Style::default().modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.disabled_fg)
        }
    }

    pub fn tab(&self, selected: bool) -> Style {
        if selected {
            Style::default().fg(self.selected_tab)
        } else {
            Style::default()
        }
    }

    pub fn text(&self, enabled: bool, selected: bool) -> Style {
        match (enabled, selected) {
            (false, _) => Style::default().fg(self.disabled_fg),
            (true, false) => Style::default(),
            (true, true) => Style::default().bg(self.selection_bg),
        }
    }

    pub fn item(&self, typ: StatusItemType, selected: bool) -> Style {
        let style = match typ {
            StatusItemType::New => {
                Style::default().fg(self.diff_file_added)
            }
            StatusItemType::Modified => {
                Style::default().fg(self.diff_file_modified)
            }
            StatusItemType::Deleted => {
                Style::default().fg(self.diff_file_removed)
            }
            StatusItemType::Renamed => {
                Style::default().fg(self.diff_file_moved)
            }
            _ => Style::default(),
        };

        self.apply_select(style, selected)
    }

    fn apply_select(&self, style: Style, selected: bool) -> Style {
        if selected {
            style.bg(self.selection_bg)
        } else {
            style
        }
    }

    pub fn diff_hunk_marker(&self, selected: bool) -> Style {
        match selected {
            false => Style::default().fg(self.disabled_fg),
            true => Style::default().bg(self.selection_bg),
        }
    }

    pub fn diff_line(
        &self,
        typ: DiffLineType,
        selected: bool,
    ) -> Style {
        let style = match typ {
            DiffLineType::Add => {
                Style::default().fg(self.diff_line_add)
            }
            DiffLineType::Delete => {
                Style::default().fg(self.diff_line_delete)
            }
            DiffLineType::Header => {
                Style::default().modifier(Modifier::BOLD)
            }
            _ => Style::default(),
        };

        self.apply_select(style, selected)
    }

    pub fn text_danger(&self) -> Style {
        Style::default().fg(self.diff_file_removed)
    }

    pub fn toolbar(&self, enabled: bool) -> Style {
        if enabled {
            Style::default().fg(self.command_fg)
        } else {
            Style::default().fg(self.disabled_fg)
        }
        .bg(self.selection_bg)
    }

    pub fn commit_hash(&self, selected: bool) -> Style {
        self.apply_select(
            Style::default().fg(self.commit_hash),
            selected,
        )
    }
    pub fn commit_time(&self, selected: bool) -> Style {
        self.apply_select(
            Style::default().fg(self.commit_time),
            selected,
        )
    }
    pub fn commit_author(&self, selected: bool) -> Style {
        self.apply_select(
            Style::default().fg(self.commit_author),
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
            let res = Self::default();
            res.save().unwrap_or_default();
            res
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            selected_tab: Color::Yellow,
            command_fg: Color::White,
            selection_bg: Color::Rgb(0, 0, 100),
            disabled_fg: Color::DarkGray,
            diff_line_add: Color::Green,
            diff_line_delete: Color::Red,
            diff_file_added: Color::LightGreen,
            diff_file_removed: Color::LightRed,
            diff_file_moved: Color::LightMagenta,
            diff_file_modified: Color::LightYellow,
            commit_hash: Color::Magenta,
            commit_time: Color::Blue,
            commit_author: Color::Green,
        }
    }
}

/// we duplicate the Color definition from `tui` crate to implement Serde serialisation
/// this enum can be removed once [tui-#292](https://github.com/fdehau/tui-rs/issues/292) is resolved
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(remote = "Color")]
enum ColorDef {
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
