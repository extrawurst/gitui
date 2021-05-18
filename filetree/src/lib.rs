// #![forbid(missing_docs)]
#![forbid(unsafe_code)]
#![deny(unused_imports)]
#![deny(unused_must_use)]
#![deny(dead_code)]
#![deny(clippy::all, clippy::perf, clippy::nursery, clippy::pedantic)]
#![deny(clippy::expect_used)]
#![deny(clippy::filetype_is_file)]
// #![deny(clippy::cargo)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::match_like_matches_macro)]
#![deny(clippy::needless_update)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]

mod error;
mod iterator;

use error::{Error, Result};
use iterator::TreeIterator;
use std::{
    collections::BTreeSet, convert::TryFrom, path::Path, usize,
};

/// holds the information shared among all `FileTreeItem` in a `FileTree`
#[derive(Debug, Clone)]
struct TreeItemInfo {
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
    const fn new(
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
}

/// attribute used to indicate the collapse/expand state of a path item
#[derive(PartialEq, Debug, Copy, Clone)]
struct PathCollapsed(pub bool);

/// `FileTreeItem` can be of two kinds
#[derive(PartialEq, Debug, Clone)]
enum FileTreeItemKind {
    Path(PathCollapsed),
    File,
}

/// `FileTreeItem` can be of two kinds: see `FileTreeItem` but shares an info
#[derive(Debug, Clone)]
struct FileTreeItem {
    info: TreeItemInfo,
    kind: FileTreeItemKind,
}

impl FileTreeItem {
    fn new_file(path: &str) -> Result<Self> {
        let item_path = Path::new(&path);

        let indent = u8::try_from(
            item_path.ancestors().count().saturating_sub(2),
        )?;

        let filename = item_path.file_name().map_or_else(
            || Err(Error::InvalidFilePath(path.to_string())),
            Ok,
        )?;

        let filename = filename.to_string_lossy().to_string();

        Ok(Self {
            info: TreeItemInfo::new(
                indent,
                filename.clone(),
                item_path.to_string_lossy().to_string(),
            ),
            kind: FileTreeItemKind::File,
        })
    }

    fn new_path(
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

    fn is_path(&self) -> bool {
        matches!(self.kind, FileTreeItemKind::Path(_))
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

///
#[derive(Default)]
pub struct FileTree {
    items: Vec<FileTreeItem>,
    file_count: usize,
}

impl FileTree {
    ///
    pub fn new(
        list: &[&str],
        collapsed: &BTreeSet<&String>,
    ) -> Result<Self> {
        let mut items = Vec::with_capacity(list.len());
        let mut paths_added = BTreeSet::new();

        for e in list {
            {
                let item_path = Path::new(e);
                Self::push_dirs(
                    item_path,
                    &mut items,
                    &mut paths_added,
                    collapsed,
                )?;
            }

            items.push(FileTreeItem::new_file(e)?);
        }

        Ok(Self {
            items,
            file_count: list.len(),
        })
    }

    /// how many individual items (files/paths) are in the list
    pub fn len(&self) -> usize {
        self.items.len()
    }

    ///
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// how many files were added to this list
    pub const fn file_count(&self) -> usize {
        self.file_count
    }

    /// iterates visible elements
    pub fn iterate(
        &self,
        start: usize,
        amount: usize,
    ) -> TreeIterator<'_> {
        TreeIterator::new(&self, start, amount)
    }

    ///
    // pub(crate) fn find_parent_index(&self, index: usize) -> usize {
    //     let item_indent = &self.items[index].info.indent;
    //     let mut parent_index = index;
    //     while item_indent <= &self.items[parent_index].info.indent {
    //         if parent_index == 0 {
    //             return 0;
    //         }
    //         parent_index -= 1;
    //     }

    //     parent_index
    // }

    fn push_dirs<'a>(
        item_path: &'a Path,
        nodes: &mut Vec<FileTreeItem>,
        paths_added: &mut BTreeSet<&'a Path>,
        collapsed: &BTreeSet<&String>,
    ) -> Result<()> {
        let mut ancestors =
            { item_path.ancestors().skip(1).collect::<Vec<_>>() };
        ancestors.reverse();

        for c in &ancestors {
            if c.parent().is_some() && !paths_added.contains(c) {
                paths_added.insert(c);
                let path_string = Self::path_to_string(c)?;
                let is_collapsed = collapsed.contains(&path_string);
                nodes.push(FileTreeItem::new_path(
                    c,
                    path_string,
                    is_collapsed,
                )?);
            }
        }

        Ok(())
    }

    fn path_to_string(p: &Path) -> Result<String> {
        Ok(p.to_str()
            .map_or_else(
                || Err(Error::InvalidPath(p.to_path_buf())),
                Ok,
            )?
            .to_string())
    }

    pub fn collapse(&mut self, index: usize) {
        if self.items[index].is_path() {
            self.items[index].kind =
                FileTreeItemKind::Path(PathCollapsed(true));

            let path =
                format!("{}/", self.items[index].info.full_path);

            for i in index + 1..self.items.len() {
                let item = &mut self.items[i];
                let item_path = &item.info.full_path;
                if item_path.starts_with(&path) {
                    item.info.visible = false
                } else {
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_simple() {
        let items = vec![
            "file.txt", //
        ];

        let res = FileTree::new(&items, &BTreeSet::new()).unwrap();

        assert_eq!(
            res.items,
            vec![FileTreeItem {
                info: TreeItemInfo {
                    path: items[0].to_string(),
                    full_path: items[0].to_string(),
                    indent: 0,
                    visible: true,
                },
                kind: FileTreeItemKind::File
            }]
        );

        let items = vec![
            "file.txt",  //
            "file2.txt", //
        ];

        let res = FileTree::new(&items, &BTreeSet::new()).unwrap();

        assert_eq!(res.items.len(), 2);
        assert_eq!(res.items.len(), res.len());
        assert_eq!(res.items[1].info.path, items[1].to_string());
    }

    #[test]
    fn test_folder() {
        let items = vec![
            "a/file.txt", //
        ];

        let res = FileTree::new(&items, &BTreeSet::new())
            .unwrap()
            .items
            .iter()
            .map(|i| i.info.full_path.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            res,
            vec![String::from("a"), String::from("a/file.txt"),]
        );
    }

    #[test]
    fn test_indent() {
        let items = vec![
            "a/b/file.txt", //
        ];

        let list = FileTree::new(&items, &BTreeSet::new()).unwrap();
        let mut res = list
            .items
            .iter()
            .map(|i| (i.info.indent, i.info.path.as_str()));

        assert_eq!(res.next(), Some((0, "a")));
        assert_eq!(res.next(), Some((1, "b")));
        assert_eq!(res.next(), Some((2, "file.txt")));
    }

    #[test]
    fn test_indent_folder_file_name() {
        let items = vec![
            "a/b",   //
            "a.txt", //
        ];

        let list = FileTree::new(&items, &BTreeSet::new()).unwrap();
        let mut res = list
            .items
            .iter()
            .map(|i| (i.info.indent, i.info.path.as_str()));

        assert_eq!(res.next(), Some((0, "a")));
        assert_eq!(res.next(), Some((1, "b")));
        assert_eq!(res.next(), Some((0, "a.txt")));
    }

    #[test]
    fn test_folder_dup() {
        let items = vec![
            "a/file.txt",  //
            "a/file2.txt", //
        ];

        let tree = FileTree::new(&items, &BTreeSet::new()).unwrap();

        assert_eq!(tree.file_count(), 2);
        assert_eq!(tree.len(), 3);

        let res = tree
            .items
            .iter()
            .map(|i| i.info.full_path.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            res,
            vec![
                String::from("a"),
                String::from("a/file.txt"),
                String::from("a/file2.txt"),
            ]
        );
    }

    #[test]
    fn test_collapse() {
        let items = vec![
            "a/file1.txt", //
            "b/file2.txt", //
        ];

        let mut tree =
            FileTree::new(&items, &BTreeSet::new()).unwrap();

        assert!(tree.items[1].info.visible);

        tree.collapse(0);

        assert!(!tree.items[1].info.visible);
    }

    #[test]
    fn test_iterate_collapsed() {
        let items = vec![
            "a/file1.txt", //
            "b/file2.txt", //
        ];

        let mut tree =
            FileTree::new(&items, &BTreeSet::new()).unwrap();

        tree.collapse(0);

        let mut it = tree.iterate(0, 10);

        assert_eq!(it.next(), Some(0));
        assert_eq!(it.next(), Some(2));
        assert_eq!(it.next(), Some(3));
        assert_eq!(it.next(), None);
    }
}
