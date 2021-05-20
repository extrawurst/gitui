use crate::{
    error::Error,
    item::{FileTreeItemKind, PathCollapsed},
    FileTreeItem,
};
use crate::{error::Result, treeitems_iter::TreeItemsIterator};
use std::{collections::BTreeSet, path::Path};

///
#[derive(Default)]
pub struct FileTreeItems {
    pub tree_items: Vec<FileTreeItem>,
    files: usize,
}

impl FileTreeItems {
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
            tree_items: items,
            files: list.len(),
        })
    }

    /// how many individual items (files/paths) are in the list
    pub fn len(&self) -> usize {
        self.tree_items.len()
    }

    ///
    pub fn is_empty(&self) -> bool {
        self.tree_items.is_empty()
    }

    /// how many files were added to this list
    pub const fn file_count(&self) -> usize {
        self.files
    }

    /// iterates visible elements
    pub const fn iterate(
        &self,
        start: usize,
        amount: usize,
    ) -> TreeItemsIterator<'_> {
        TreeItemsIterator::new(self, start, amount)
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
        if self.tree_items[index].kind().is_path() {
            self.tree_items[index].collapse_path();

            let path = format!(
                "{}/",
                self.tree_items[index].info().full_path()
            );

            for i in index + 1..self.tree_items.len() {
                let item = &mut self.tree_items[i];

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
        if self.tree_items[index].kind().is_path() {
            self.tree_items[index].expand_path();
            let full_path = format!(
                "{}/",
                self.tree_items[index].info().full_path()
            );

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

        for i in start_idx..self.tree_items.len() {
            if let Some(ref collapsed_path) = inner_collapsed {
                let p = self.tree_items[i].info().full_path();
                if p.starts_with(collapsed_path) {
                    if set_defaults {
                        self.tree_items[i]
                            .info_mut()
                            .set_visible(false);
                    }
                    // we are still in a collapsed inner path
                    continue;
                }
                inner_collapsed = None;
            }

            let item_kind = self.tree_items[i].kind().clone();
            let item_path = self.tree_items[i].info().full_path();

            if matches!(item_kind, FileTreeItemKind::Path(PathCollapsed(collapsed)) if collapsed)
            {
                // we encountered an inner path that is still collapsed
                inner_collapsed = Some(format!("{}/", &item_path));
            }

            if prefix
                .map_or(true, |prefix| item_path.starts_with(prefix))
            {
                self.tree_items[i].info_mut().set_visible(true);
            } else {
                // if we do not set defaults we can early out
                if set_defaults {
                    self.tree_items[i].info_mut().set_visible(false);
                } else {
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_simple() {
        let items = vec![
            "file.txt", //
        ];

        let res =
            FileTreeItems::new(&items, &BTreeSet::new()).unwrap();

        assert!(res.tree_items[0].info().is_visible());
        assert_eq!(res.tree_items[0].info().indent(), 0);
        assert_eq!(res.tree_items[0].info().path(), items[0]);
        assert_eq!(res.tree_items[0].info().full_path(), items[0]);

        let items = vec![
            "file.txt",  //
            "file2.txt", //
        ];

        let res =
            FileTreeItems::new(&items, &BTreeSet::new()).unwrap();

        assert_eq!(res.tree_items.len(), 2);
        assert_eq!(res.tree_items.len(), res.len());
        assert_eq!(
            res.tree_items[1].info().path(),
            items[1].to_string()
        );
    }

    #[test]
    fn test_folder() {
        let items = vec![
            "a/file.txt", //
        ];

        let res = FileTreeItems::new(&items, &BTreeSet::new())
            .unwrap()
            .tree_items
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

        let list =
            FileTreeItems::new(&items, &BTreeSet::new()).unwrap();
        let mut res = list
            .tree_items
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

        let list =
            FileTreeItems::new(&items, &BTreeSet::new()).unwrap();
        let mut res = list
            .tree_items
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

        let tree =
            FileTreeItems::new(&items, &BTreeSet::new()).unwrap();

        assert_eq!(tree.file_count(), 2);
        assert_eq!(tree.len(), 3);

        let res = tree
            .tree_items
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
            FileTreeItems::new(&items, &BTreeSet::new()).unwrap();

        assert!(tree.tree_items[1].info().is_visible());

        tree.collapse(0, false);

        assert!(!tree.tree_items[1].info().is_visible());
    }

    #[test]
    fn test_iterate_collapsed() {
        let items = vec![
            "a/file1.txt", //
            "b/file2.txt", //
        ];

        let mut tree =
            FileTreeItems::new(&items, &BTreeSet::new()).unwrap();

        tree.collapse(0, false);

        let mut it = tree.iterate(0, 10);

        assert_eq!(it.next().unwrap().0, 0);
        assert_eq!(it.next().unwrap().0, 2);
        assert_eq!(it.next().unwrap().0, 3);
        assert_eq!(it.next(), None);
    }

    pub fn get_visibles(tree: &FileTreeItems) -> Vec<bool> {
        tree.tree_items
            .iter()
            .map(|e| e.info().is_visible())
            .collect::<Vec<_>>()
    }

    #[test]
    fn test_expand() {
        let items = vec![
            "a/b/c", //
            "a/d",   //
        ];

        //0 a/
        //1   b/
        //2     c
        //3   d

        let mut tree =
            FileTreeItems::new(&items, &BTreeSet::new()).unwrap();

        tree.collapse(1, false);

        let visibles = get_visibles(&tree);

        assert_eq!(
            visibles,
            vec![
                true,  //
                true,  //
                false, //
                true,
            ]
        );

        tree.expand(1);

        let visibles = get_visibles(&tree);

        assert_eq!(
            visibles,
            vec![
                true, //
                true, //
                true, //
                true,
            ]
        );
    }

    #[test]
    fn test_expand_bug() {
        let items = vec![
            "a/b/c",  //
            "a/b2/d", //
        ];

        //0 a/
        //1   b/
        //2     c
        //3   b2/
        //4     d

        let mut tree =
            FileTreeItems::new(&items, &BTreeSet::new()).unwrap();

        tree.collapse(1, false);
        tree.collapse(0, false);

        assert_eq!(
            get_visibles(&tree),
            vec![
                true,  //
                false, //
                false, //
                false, //
                false,
            ]
        );

        tree.expand(0);

        assert_eq!(
            get_visibles(&tree),
            vec![
                true,  //
                true,  //
                false, //
                true,  //
                true,
            ]
        );
    }

    #[test]
    fn test_collapse_too_much() {
        let items = vec![
            "a/b",  //
            "a2/c", //
        ];

        //0 a/
        //1   b
        //2 a2/
        //3   c

        let mut tree =
            FileTreeItems::new(&items, &BTreeSet::new()).unwrap();

        tree.collapse(0, false);

        let visibles = get_visibles(&tree);

        assert_eq!(
            visibles,
            vec![
                true,  //
                false, //
                true,  //
                true,
            ]
        );
    }

    #[test]
    fn test_expand_with_collapsed_sub_parts() {
        let items = vec![
            "a/b/c", //
            "a/d",   //
        ];

        //0 a/
        //1   b/
        //2     c
        //3   d

        let mut tree =
            FileTreeItems::new(&items, &BTreeSet::new()).unwrap();

        tree.collapse(1, false);

        let visibles = get_visibles(&tree);

        assert_eq!(
            visibles,
            vec![
                true,  //
                true,  //
                false, //
                true,
            ]
        );

        tree.collapse(0, false);

        let visibles = get_visibles(&tree);

        assert_eq!(
            visibles,
            vec![
                true,  //
                false, //
                false, //
                false,
            ]
        );

        tree.expand(0);

        let visibles = get_visibles(&tree);

        assert_eq!(
            visibles,
            vec![
                true,  //
                true,  //
                false, //
                true,
            ]
        );
    }
}
