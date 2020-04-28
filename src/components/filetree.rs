use asyncgit::StatusItem;
use std::{
    collections::{BTreeSet, BinaryHeap},
    convert::TryFrom,
    ops::{Index, IndexMut},
    path::Path,
};

/// holds the information shared among all `FileTreeItem` in a `FileTree`
#[derive(Debug, Clone)]
pub struct TreeItemInfo {
    /// indent level
    pub indent: u8,
    /// currently visible depending on the folder collapse states
    pub visible: bool,
    /// just the last path element
    pub path: String,
    /// the full path
    pub full_path: String,
}

impl TreeItemInfo {
    fn new(indent: u8, path: String, full_path: String) -> Self {
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
pub struct PathCollapsed(pub bool);

/// `FileTreeItem` can be of two kinds
#[derive(PartialEq, Debug, Clone)]
pub enum FileTreeItemKind {
    Path(PathCollapsed),
    File(StatusItem),
}

/// `FileTreeItem` can be of two kinds: see `FileTreeItem` but shares an info
#[derive(Debug, Clone)]
pub struct FileTreeItem {
    pub info: TreeItemInfo,
    pub kind: FileTreeItemKind,
}

impl FileTreeItem {
    fn new_file(item: &StatusItem) -> Self {
        let item_path = Path::new(&item.path);
        let indent = u8::try_from(
            item_path.ancestors().count().saturating_sub(2),
        )
        .unwrap();
        let path = String::from(
            item_path.file_name().unwrap().to_str().unwrap(),
        );

        Self {
            info: TreeItemInfo::new(indent, path, item.path.clone()),
            kind: FileTreeItemKind::File(item.clone()),
        }
    }

    fn new_path(
        path: &Path,
        path_string: String,
        collapsed: bool,
    ) -> Self {
        let indent =
            u8::try_from(path.ancestors().count().saturating_sub(2))
                .unwrap();
        let path = String::from(
            path.components()
                .last()
                .unwrap()
                .as_os_str()
                .to_str()
                .unwrap(),
        );

        Self {
            info: TreeItemInfo::new(indent, path, path_string),
            kind: FileTreeItemKind::Path(PathCollapsed(collapsed)),
        }
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
pub struct FileTreeItems(Vec<FileTreeItem>);

impl FileTreeItems {
    ///
    pub(crate) fn new(
        list: &[StatusItem],
        collapsed: &BTreeSet<&String>,
    ) -> Self {
        let mut nodes = BinaryHeap::with_capacity(list.len());
        let mut paths_added = BTreeSet::new();

        for e in list {
            let item_path = Path::new(&e.path);

            FileTreeItems::push_dirs(
                item_path,
                &mut nodes,
                &mut paths_added,
                &collapsed,
            );

            nodes.push(FileTreeItem::new_file(e));
        }

        Self(nodes.into_sorted_vec())
    }

    ///
    pub(crate) fn items(&self) -> &Vec<FileTreeItem> {
        &self.0
    }

    ///
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    fn push_dirs(
        item_path: &Path,
        nodes: &mut BinaryHeap<FileTreeItem>,
        paths_added: &mut BTreeSet<String>, //TODO: use a ref string here
        collapsed: &BTreeSet<&String>,
    ) {
        for c in item_path.ancestors().skip(1) {
            if c.parent().is_some() {
                let path_string = String::from(c.to_str().unwrap());
                if !paths_added.contains(&path_string) {
                    paths_added.insert(path_string.clone());
                    let is_collapsed =
                        collapsed.contains(&path_string);
                    nodes.push(FileTreeItem::new_path(
                        c,
                        path_string,
                        is_collapsed,
                    ));
                }
            }
        }
    }
}

impl IndexMut<usize> for FileTreeItems {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.0[idx]
    }
}

impl Index<usize> for FileTreeItems {
    type Output = FileTreeItem;

    fn index(&self, idx: usize) -> &Self::Output {
        &self.0[idx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string_vec_to_status(items: &[&str]) -> Vec<StatusItem> {
        items
            .iter()
            .map(|a| StatusItem {
                path: String::from(*a),
                status: None,
            })
            .collect::<Vec<_>>()
    }

    #[test]
    fn test_simple() {
        let items = string_vec_to_status(&[
            "file.txt", //
        ]);

        let res = FileTreeItems::new(&items, &BTreeSet::new());

        assert_eq!(
            res.0,
            vec![FileTreeItem {
                info: TreeItemInfo {
                    path: items[0].path.clone(),
                    full_path: items[0].path.clone(),
                    indent: 0,
                    visible: true,
                },
                kind: FileTreeItemKind::File(items[0].clone())
            }]
        );

        let items = string_vec_to_status(&[
            "file.txt",  //
            "file2.txt", //
        ]);

        let res = FileTreeItems::new(&items, &BTreeSet::new());

        assert_eq!(res.0.len(), 2);
        assert_eq!(res.0[1].info.path, items[1].path);
    }

    #[test]
    fn test_folder() {
        let items = string_vec_to_status(&[
            "a/file.txt", //
        ]);

        let res = FileTreeItems::new(&items, &BTreeSet::new())
            .0
            .iter()
            .map(|i| i.info.full_path.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            res,
            vec![String::from("a"), items[0].path.clone(),]
        );
    }

    #[test]
    fn test_indent() {
        let items = string_vec_to_status(&[
            "a/b/file.txt", //
        ]);

        let list = FileTreeItems::new(&items, &BTreeSet::new());
        let mut res = list
            .0
            .iter()
            .map(|i| (i.info.indent, i.info.path.as_str()));

        assert_eq!(res.next(), Some((0, "a")));
        assert_eq!(res.next(), Some((1, "b")));
        assert_eq!(res.next(), Some((2, "file.txt")));
    }

    #[test]
    fn test_folder_dup() {
        let items = string_vec_to_status(&[
            "a/file.txt",  //
            "a/file2.txt", //
        ]);

        let res = FileTreeItems::new(&items, &BTreeSet::new())
            .0
            .iter()
            .map(|i| i.info.full_path.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            res,
            vec![
                String::from("a"),
                items[0].path.clone(),
                items[1].path.clone()
            ]
        );
    }
}
