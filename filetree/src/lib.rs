// #![forbid(missing_docs)]
#![forbid(unsafe_code)]
#![deny(unused_imports)]
#![deny(unused_must_use)]
#![deny(dead_code)]
#![deny(clippy::all, clippy::perf, clippy::nursery, clippy::pedantic)]
#![deny(clippy::expect_used)]
#![deny(clippy::filetype_is_file)]
#![deny(clippy::cargo)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::match_like_matches_macro)]
#![deny(clippy::needless_update)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]

mod error;
mod item;
mod iterator;

use error::{Error, Result};
use iterator::TreeIterator;
use std::{collections::BTreeSet, path::Path, usize};

pub use item::{FileTreeItem, TreeItemInfo};

use crate::item::{FileTreeItemKind, PathCollapsed};

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
    pub const fn iterate(
        &self,
        start: usize,
        amount: usize,
    ) -> TreeIterator<'_> {
        TreeIterator::new(self, start, amount)
    }

    fn push_dirs<'a>(
        item_path: &'a Path,
        nodes: &mut Vec<FileTreeItem>,
        paths_added: &mut BTreeSet<&'a Path>,
        collapsed: &BTreeSet<&String>,
    ) -> Result<()> {
        let mut ancestors =
            item_path.ancestors().skip(1).collect::<Vec<_>>();
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

    pub fn collapse(&mut self, index: usize, recursive: bool) {
        if self.items[index].kind().is_path() {
            self.items[index].collapse_path();

            let path =
                format!("{}/", self.items[index].info().full_path());

            for i in index + 1..self.items.len() {
                let item = &mut self.items[i];

                if recursive && item.kind().is_path() {
                    item.collapse_path();
                }

                let item_path = &item.info().full_path();

                if item_path.starts_with(&path) {
                    item.hide();
                } else {
                    return;
                }
            }
        }
    }

    pub fn expand(&mut self, index: usize) {
        if self.items[index].kind().is_path() {
            self.items[index].expand_path();
            let full_path =
                format!("{}/", self.items[index].info().full_path());

            self.update_visibility(
                Some(full_path.as_str()),
                index + 1,
                false,
            );
        }
    }

    fn update_visibility(
        &mut self,
        prefix: Option<&str>,
        start_idx: usize,
        set_defaults: bool,
    ) {
        // if we are in any subpath that is collapsed we keep skipping over it
        let mut inner_collapsed: Option<String> = None;

        for i in start_idx..self.items.len() {
            if let Some(ref collapsed_path) = inner_collapsed {
                let p = self.items[i].info().full_path();
                if p.starts_with(collapsed_path) {
                    if set_defaults {
                        self.items[i].info_mut().set_visible(false);
                    }
                    // we are still in a collapsed inner path
                    continue;
                }
                inner_collapsed = None;
            }

            let item_kind = self.items[i].kind().clone();
            let item_path = self.items[i].info().full_path();

            if matches!(item_kind, FileTreeItemKind::Path(PathCollapsed(collapsed)) if collapsed)
            {
                // we encountered an inner path that is still collapsed
                inner_collapsed = Some(format!("{}/", &item_path));
            }

            if prefix
                .map_or(true, |prefix| item_path.starts_with(prefix))
            {
                self.items[i].info_mut().set_visible(true);
            } else {
                // if we do not set defaults we can early out
                if set_defaults {
                    self.items[i].info_mut().set_visible(false);
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

        assert!(res.items[0].info().is_visible());
        assert_eq!(res.items[0].info().indent(), 0);
        assert_eq!(res.items[0].info().path(), items[0]);
        assert_eq!(res.items[0].info().full_path(), items[0]);

        let items = vec![
            "file.txt",  //
            "file2.txt", //
        ];

        let res = FileTree::new(&items, &BTreeSet::new()).unwrap();

        assert_eq!(res.items.len(), 2);
        assert_eq!(res.items.len(), res.len());
        assert_eq!(res.items[1].info().path(), items[1].to_string());
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
            .map(|i| i.info().full_path().to_string())
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
            .map(|i| (i.info().indent(), i.info().path()));

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
            .map(|i| (i.info().indent(), i.info().path()));

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
            .map(|i| i.info().full_path().to_string())
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

        assert!(tree.items[1].info().is_visible());

        tree.collapse(0, false);

        assert!(!tree.items[1].info().is_visible());
    }

    #[test]
    fn test_iterate_collapsed() {
        let items = vec![
            "a/file1.txt", //
            "b/file2.txt", //
        ];

        let mut tree =
            FileTree::new(&items, &BTreeSet::new()).unwrap();

        tree.collapse(0, false);

        let mut it = tree.iterate(0, 10);

        assert_eq!(it.next().unwrap().0, 0);
        assert_eq!(it.next().unwrap().0, 2);
        assert_eq!(it.next().unwrap().0, 3);
        assert_eq!(it.next(), None);
    }
}
