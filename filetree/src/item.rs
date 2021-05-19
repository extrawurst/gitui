use crate::error::{Error, Result};
use std::{convert::TryFrom, path::Path};

/// holds the information shared among all `FileTreeItem` in a `FileTree`
#[derive(Debug, Clone)]
pub struct TreeItemInfo {
    /// indent level
    indent: u8,
    /// currently visible depending on the folder collapse states
    visible: bool,
    /// just the last path element
    path: String,
    /// the full path
    full_path: String,
}

impl TreeItemInfo {
    ///
    pub const fn new(
        indent: u8,
        path: String,
        full_path: String,
    ) -> Self {
        Self {
            indent,
            visible: true,
            path,
            full_path,
        }
    }

    ///
    pub const fn is_visible(&self) -> bool {
        self.visible
    }

    ///
    pub fn full_path(&self) -> &str {
        &self.full_path
    }

    ///
    #[cfg(test)]
    pub fn path(&self) -> &str {
        &self.path
    }

    ///
    #[cfg(test)]
    pub fn indent(&self) -> u8 {
        self.indent
    }
}

/// attribute used to indicate the collapse/expand state of a path item
#[derive(PartialEq, Debug, Copy, Clone)]
pub struct PathCollapsed(pub bool);

/// `FileTreeItem` can be of two kinds
#[derive(PartialEq, Debug, Clone)]
pub enum FileTreeItemKind {
    Path(PathCollapsed),
    File,
}

impl FileTreeItemKind {
    pub const fn is_path(&self) -> bool {
        matches!(self, FileTreeItemKind::Path(_))
    }
}

/// `FileTreeItem` can be of two kinds: see `FileTreeItem` but shares an info
#[derive(Debug, Clone)]
pub struct FileTreeItem {
    info: TreeItemInfo,
    kind: FileTreeItemKind,
}

impl FileTreeItem {
    pub fn new_file(path: &str) -> Result<Self> {
        let item_path = Path::new(&path);

        let indent = u8::try_from(
            item_path.ancestors().count().saturating_sub(2),
        )?;

        let filename = item_path
            .file_name()
            .map_or_else(
                || Err(Error::InvalidFilePath(path.to_string())),
                Ok,
            )?
            .to_string_lossy()
            .to_string();

        Ok(Self {
            info: TreeItemInfo::new(
                indent,
                filename,
                item_path.to_string_lossy().to_string(),
            ),
            kind: FileTreeItemKind::File,
        })
    }

    pub fn new_path(
        path: &Path,
        path_string: String,
        collapsed: bool,
    ) -> Result<Self> {
        let indent =
            u8::try_from(path.ancestors().count().saturating_sub(2))?;

        let last_path_component =
            path.components().last().map_or_else(
                || Err(Error::InvalidPath(path.to_path_buf())),
                Ok,
            )?;
        let last_path_component = last_path_component
            .as_os_str()
            .to_string_lossy()
            .to_string();

        Ok(Self {
            info: TreeItemInfo::new(
                indent,
                last_path_component,
                path_string,
            ),
            kind: FileTreeItemKind::Path(PathCollapsed(collapsed)),
        })
    }

    ///
    pub const fn info(&self) -> &TreeItemInfo {
        &self.info
    }

    ///
    pub const fn kind(&self) -> &FileTreeItemKind {
        &self.kind
    }

    ///
    pub fn collapse_path(&mut self) {
        self.kind = FileTreeItemKind::Path(PathCollapsed(true));
    }

    ///
    pub fn hide(&mut self) {
        self.info.visible = false;
    }
}

impl Eq for FileTreeItem {}

impl PartialEq for FileTreeItem {
    fn eq(&self, other: &Self) -> bool {
        self.info.full_path.eq(&other.info.full_path)
    }
}

impl PartialOrd for FileTreeItem {
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<std::cmp::Ordering> {
        self.info.full_path.partial_cmp(&other.info.full_path)
    }
}

impl Ord for FileTreeItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.info.path.cmp(&other.info.path)
    }
}
