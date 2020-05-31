use crate::get_app_config_path;
use anyhow::Result;
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
    cmdbar_extra_lines_bg: Color,
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
    #[serde(with = "ColorDef")]
    danger_fg: Color,
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
            (true, true) => Style::default()
                .fg(self.command_fg)
                .bg(self.selection_bg),
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

    pub fn option(&self, on: bool) -> Style {
        if on {
            Style::default().fg(self.diff_line_add)
        } else {
            Style::default().fg(self.diff_line_delete)
        }
    }

    pub fn diff_hunk_marker(&self, selected: bool) -> Style {
        if selected {
            Style::default().bg(self.selection_bg)
        } else {
            Style::default().fg(self.disabled_fg)
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
            DiffLineType::Header => Style::default()
                .fg(self.disabled_fg)
                .modifier(Modifier::BOLD),
            _ => Style::default().fg(if selected {
                self.command_fg
            } else {
                Color::Reset
            }),
        };

        self.apply_select(style, selected)
    }

    pub fn text_danger(&self) -> Style {
        Style::default().fg(self.danger_fg)
    }

    pub fn commandbar(&self, enabled: bool, line: usize) -> Style {
        if enabled {
            Style::default().fg(self.command_fg)
        } else {
            Style::default().fg(self.disabled_fg)
        }
        .bg(if line == 0 {
            self.selection_bg
        } else {
            self.cmdbar_extra_lines_bg
        })
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

    fn save(&self) -> Result<()> {
        let theme_file = Self::get_theme_file()?;
        let mut file = File::create(theme_file)?;
        let data = to_string_pretty(self, PrettyConfig::default())?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }

    fn get_theme_file() -> Result<PathBuf> {
        let app_home = get_app_config_path()?;
        Ok(app_home.join("theme.ron"))
    }

    fn read_file(theme_file: PathBuf) -> Result<Self> {
        let mut f = File::open(theme_file)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        Ok(from_bytes(&buffer)?)
    }

    fn init_internal() -> Result<Self> {
        let file = Self::get_theme_file()?;
        if file.exists() {
            Ok(Self::read_file(file)?)
        } else {
            let def = Self::default();
            if def.save().is_err() {
                log::warn!("failed to store default theme to disk.")
            }
            Ok(def)
        }
    }

    pub fn init() -> Self {
        Self::init_internal().unwrap_or_default()
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            selected_tab: Color::Yellow,
            command_fg: Color::White,
            selection_bg: Color::Rgb(0, 0, 100),
            cmdbar_extra_lines_bg: Color::Rgb(0, 0, 80),
            disabled_fg: Color::DarkGray,
            diff_line_add: Color::Green,
            diff_line_delete: Color::Red,
            diff_file_added: Color::LightGreen,
            diff_file_removed: Color::LightRed,
            diff_file_moved: Color::LightMagenta,
            diff_file_modified: Color::Yellow,
            commit_hash: Color::Magenta,
            commit_time: Color::Rgb(110, 110, 255),
            commit_author: Color::Green,
            danger_fg: Color::Red,
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
