use crate::{
    error::Result, filetreeitems::FileTreeItems,
    tree_iter::TreeIterator,
};
use std::collections::BTreeSet;

///
#[derive(Copy, Clone, Debug)]
pub enum MoveSelection {
    Up,
    Down,
    // Left,
    // Right,
    // Home,
    // End,
}

/// wraps `FileTreeItems` as a datastore and adds selection functionality
#[derive(Default)]
pub struct FileTree {
    items: FileTreeItems,
    selection: Option<usize>,
}

impl FileTree {
    ///
    pub fn new(
        list: &[&str],
        collapsed: &BTreeSet<&String>,
    ) -> Result<Self> {
        let selection = if list.is_empty() { None } else { Some(0) };

        Ok(Self {
            items: FileTreeItems::new(list, collapsed)?,
            selection,
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
        self.items.file_count()
    }

    /// iterates visible elements
    pub const fn iterate(
        &self,
        start: usize,
        amount: usize,
    ) -> TreeIterator<'_> {
        TreeIterator::new(
            self.items.iterate(start, amount),
            self.selection,
        )
    }

    ///
    pub fn collapse(&mut self, index: usize, recursive: bool) {
        self.items.collapse(index, recursive);
    }

    ///
    pub fn expand(&mut self, index: usize) {
        self.items.expand(index);
    }

    ///
    pub fn move_selection(&mut self, dir: MoveSelection) -> bool {
        self.selection.map_or(false, |selection| {
            let new_index = match dir {
                MoveSelection::Up => {
                    self.selection_updown(selection, true)
                }
                MoveSelection::Down => {
                    self.selection_updown(selection, false)
                }
                // MoveSelection::Left => self.selection_left(selection),
                // MoveSelection::Right => {
                //     self.selection_right(selection)
                // }
                // MoveSelection::Home => SelectionChange::new(0, false),
                // MoveSelection::End => self.selection_end(),
            };

            let changed_index = new_index != selection;

            self.selection = Some(new_index);

            changed_index
        })
    }

    fn selection_updown(
        &self,
        current_index: usize,
        up: bool,
    ) -> usize {
        let mut index = current_index;

        loop {
            index = {
                let new_index = if up {
                    index.saturating_sub(1)
                } else {
                    index.saturating_add(1)
                };

                // when reaching usize bounds
                if new_index == index {
                    break;
                }

                if new_index >= self.len() {
                    break;
                }

                new_index
            };

            if self.is_visible_index(index) {
                break;
            }
        }

        index
    }

    fn is_visible_index(&self, index: usize) -> bool {
        self.items
            .tree_items
            .get(index)
            .map(|item| item.info().is_visible())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod test {
    use crate::{FileTree, MoveSelection};
    use pretty_assertions::assert_eq;
    use std::collections::BTreeSet;

    #[test]
    fn test_selection() {
        let items = vec![
            "a/b", //
        ];

        let mut tree =
            FileTree::new(&items, &BTreeSet::new()).unwrap();

        assert!(tree.move_selection(MoveSelection::Down));

        assert_eq!(tree.selection, Some(1));

        assert!(!tree.move_selection(MoveSelection::Down));

        assert_eq!(tree.selection, Some(1));
    }

    #[test]
    fn test_selection_skips_collapsed() {
        let items = vec![
            "a/b/c", //
            "a/d",   //
        ];

        //0 a/
        //1   b/
        //2     c
        //3   d

        let mut tree =
            FileTree::new(&items, &BTreeSet::new()).unwrap();

        tree.collapse(1, false);
        tree.selection = Some(1);

        assert!(tree.move_selection(MoveSelection::Down));

        assert_eq!(tree.selection, Some(3));
    }
}
