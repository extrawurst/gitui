//TODO: remove once fixed https://github.com/rust-lang/rust-clippy/issues/6818
#![allow(clippy::use_self)]

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
    rc::Rc,
};

use anyhow::Result;
use ron::{
    de::from_bytes,
    ser::{PrettyConfig, to_string_pretty},
};
use serde::{Deserialize, Serialize};
use tui::style::{Color, Modifier, Style};

use asyncgit::{DiffLineType, StatusItemType};

pub type SharedTheme = Rc<Theme>;

#[derive(Serialize, Deserialize, Debug)]
pub struct Theme {
    #[serde(with = "Color", default = "Theme::default_selected_tab")]
    selected_tab: Color,
    #[serde(with = "Color", default = "Theme::default_command_fg")]
    command_fg: Color,
    #[serde(with = "Color", default = "Theme::default_selection_bg")]
    selection_bg: Color,
    #[serde(with = "Color", default = "Theme::default_cmdbar_extra_lines_bg")]
    cmdbar_extra_lines_bg: Color,
    #[serde(with = "Color", default = "Theme::default_disabled_fg")]
    disabled_fg: Color,
    #[serde(with = "Color", default = "Theme::default_diff_line_add")]
    diff_line_add: Color,
    #[serde(with = "Color", default = "Theme::default_diff_line_delete")]
    diff_line_delete: Color,
    #[serde(with = "Color", default = "Theme::default_diff_file_added")]
    diff_file_added: Color,
    #[serde(with = "Color", default = "Theme::default_diff_file_removed")]
    diff_file_removed: Color,
    #[serde(with = "Color", default = "Theme::default_diff_file_moved")]
    diff_file_moved: Color,
    #[serde(with = "Color", default = "Theme::default_diff_file_modified")]
    diff_file_modified: Color,
    #[serde(with = "Color", default = "Theme::default_commit_hash")]
    commit_hash: Color,
    #[serde(with = "Color", default = "Theme::default_commit_time")]
    commit_time: Color,
    #[serde(with = "Color", default = "Theme::default_commit_author")]
    commit_author: Color,
    #[serde(with = "Color", default = "Theme::default_danger_fg")]
    danger_fg: Color,
    #[serde(with = "Color", default = "Theme::default_push_gauge_bg")]
    push_gauge_bg: Color,
    #[serde(with = "Color", default = "Theme::default_push_gauge_fg")]
    push_gauge_fg: Color,
}

impl Theme {
    pub fn scroll_bar_pos(&self) -> Style {
        Style::default().fg(self.selection_bg)
    }

    pub fn block(&self, focus: bool) -> Style {
        if focus {
            Style::default()
        } else {
            Style::default().fg(self.disabled_fg)
        }
    }

    pub fn title(&self, focused: bool) -> Style {
        if focused {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.disabled_fg)
        }
    }

    pub fn branch(&self, selected: bool, head: bool) -> Style {
        let branch = if head {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        if selected {
            branch.patch(Style::default().bg(self.selection_bg))
        } else {
            branch
        }
    }

    pub fn tab(&self, selected: bool) -> Style {
        if selected {
            self.text(true, false)
                .fg(self.selected_tab)
                .add_modifier(Modifier::UNDERLINED)
        } else {
            self.text(false, false)
        }
    }

    pub fn tags(&self, selected: bool) -> Style {
        Style::default()
            .fg(self.selected_tab)
            .add_modifier(Modifier::BOLD)
            .bg(if selected {
                self.selection_bg
            } else {
                Color::Reset
            })
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
            StatusItemType::Conflicted => Style::default()
                .fg(self.diff_file_modified)
                .add_modifier(Modifier::BOLD),
            StatusItemType::Typechange => Style::default(),
        };

        self.apply_select(style, selected)
    }

    pub fn file_tree_item(
        &self,
        is_folder: bool,
        selected: bool,
    ) -> Style {
        let style = if is_folder {
            Style::default()
        } else {
            Style::default().fg(self.diff_file_modified)
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
                .add_modifier(Modifier::BOLD),
            DiffLineType::None => Style::default().fg(if selected {
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

    pub fn commit_hash_in_blame(
        &self,
        is_blamed_commit: bool,
    ) -> Style {
        if is_blamed_commit {
            Style::default()
                .fg(self.commit_hash)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.commit_hash)
        }
    }

    pub fn push_gauge(&self) -> Style {
        Style::default()
            .fg(self.push_gauge_fg)
            .bg(self.push_gauge_bg)
    }

    // This will only be called when theme.ron doesn't already exists
    fn save(&self, theme_file: PathBuf) -> Result<()> {
        let mut file = File::create(theme_file)?;
        let data = to_string_pretty(self, PrettyConfig::default())?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }

    fn read_file(theme_file: PathBuf) -> Result<Self> {
        let mut f = File::open(theme_file)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        Ok(from_bytes(&buffer)?)
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

    fn default_selected_tab() -> Color { Color::Reset }
    fn default_command_fg() -> Color { Color::White }
    fn default_selection_bg() -> Color { Color::Blue }
    fn default_cmdbar_extra_lines_bg() -> Color { Color::Blue }
    fn default_disabled_fg() -> Color { Color::DarkGray }
    fn default_diff_line_add() -> Color { Color::Green }
    fn default_diff_line_delete() -> Color { Color::Red }
    fn default_diff_file_added() -> Color { Color::LightGreen }
    fn default_diff_file_removed() -> Color { Color::LightRed }
    fn default_diff_file_moved() -> Color { Color::LightMagenta }
    fn default_diff_file_modified() -> Color { Color::Yellow }
    fn default_commit_hash() -> Color { Color::Magenta }
    fn default_commit_time() -> Color { Color::LightCyan }
    fn default_commit_author() -> Color { Color::Green }
    fn default_danger_fg() -> Color { Color::Red }
    fn default_push_gauge_bg() -> Color { Color::Blue }
    fn default_push_gauge_fg() -> Color { Color::Reset }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            selected_tab: Self::default_selected_tab(),
            command_fg: Self::default_command_fg(),
            selection_bg: Self::default_selection_bg(),
            cmdbar_extra_lines_bg: Self::default_cmdbar_extra_lines_bg(),
            disabled_fg: Self::default_disabled_fg(),
            diff_line_add: Self::default_diff_line_add(),
            diff_line_delete: Self::default_diff_line_delete(),
            diff_file_added: Self::default_diff_file_added(),
            diff_file_removed: Self::default_diff_file_removed(),
            diff_file_moved: Self::default_diff_file_moved(),
            diff_file_modified: Self::default_diff_file_modified(),
            commit_hash: Self::default_commit_hash(),
            commit_time: Self::default_commit_time(),
            commit_author: Self::default_commit_author(),
            danger_fg: Self::default_danger_fg(),
            push_gauge_bg: Self::default_push_gauge_bg(),
            push_gauge_fg: Self::default_push_gauge_fg(),
        }
    }
}

#[cfg(test)]
mod tests {
    use tui::style::Color;

    use super::Theme;

    #[test]
    fn test_theme_fields_use_default_values_if_missing_during_deserialization() {
        let input = "( selection_bg: Green, )";
        let result: ron::Result<Theme> = ron::de::from_str(input);

        assert_eq!(result.is_ok(), true);
        let config = result.unwrap();

        let expected_selection_bg = Color::Green;
        assert_eq!(config.selection_bg, expected_selection_bg);

        let expected_command_fg = Color::White;
        assert_eq!(config.command_fg, expected_command_fg);
    }
}
