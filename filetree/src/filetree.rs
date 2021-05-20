use crate::{
    error::Result, filetreeitems::FileTreeItems,
    tree_iter::TreeIterator,
};
use std::{collections::BTreeSet, usize};

///
#[derive(Copy, Clone, Debug)]
pub enum MoveSelection {
    Up,
    Down,
    Left,
    Right,
    Top,
    End,
}

pub struct VisualSelection {
    pub count: usize,
    pub index: usize,
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

    ///
    pub fn collapse_but_root(&mut self) {
        self.items.collapse(0, true);
        self.items.expand(0);
    }

    /// iterates visible elements starting from `start_index_visual`
    pub fn iterate(
        &self,
        start_index_visual: usize,
        max_amount: usize,
    ) -> TreeIterator<'_> {
        let start = self
            .visual_index_to_absolute(start_index_visual)
            .unwrap_or_default();
        TreeIterator::new(
            self.items.iterate(start, max_amount),
            self.selection,
        )
    }

    fn visual_index_to_absolute(
        &self,
        visual_index: usize,
    ) -> Option<usize> {
        self.items
            .iterate(0, self.items.len())
            .enumerate()
            .find_map(|(i, (abs, _))| {
                if i == visual_index {
                    Some(abs)
                } else {
                    None
                }
            })
    }

    ///
    //TODO:cache
    pub fn visual_selection(&self) -> VisualSelection {
        let mut count = 0;
        let mut selection = 0;
        for (index, _item) in self.items.iterate(0, self.items.len())
        {
            if self
                .selection
                .map(|selection| selection == index)
                .unwrap_or_default()
            {
                selection = count;
            }

            count += 1;
        }

        VisualSelection {
            index: selection,
            count,
        }
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
                MoveSelection::Left => self.selection_left(selection),
                MoveSelection::Right => {
                    self.selection_right(selection)
                }
                MoveSelection::Top => {
                    Self::selection_start(selection)
                }
                MoveSelection::End => self.selection_end(selection),
            };

            let changed_index =
                new_index.map(|i| i != selection).unwrap_or_default();

            if changed_index {
                self.selection = new_index;
            }

            changed_index || new_index.is_some()
        })
    }

    const fn selection_start(current_index: usize) -> Option<usize> {
        if current_index == 0 {
            None
        } else {
            Some(0)
        }
    }

    fn selection_end(&self, current_index: usize) -> Option<usize> {
        let items_max = self.items.len().saturating_sub(1);

        let mut new_index = items_max;

        loop {
            if self.is_visible_index(new_index) {
                break;
            }

            if new_index == 0 {
                break;
            }

            new_index = new_index.saturating_sub(1);
            new_index = std::cmp::min(new_index, items_max);
        }

        if new_index == current_index {
            None
        } else {
            Some(new_index)
        }
    }

    fn selection_updown(
        &self,
        current_index: usize,
        up: bool,
    ) -> Option<usize> {
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

                if new_index >= self.items.len() {
                    break;
                }

                new_index
            };

            if self.is_visible_index(index) {
                break;
            }
        }

        if index == current_index {
            None
        } else {
            Some(index)
        }
    }

    fn select_parent(
        &mut self,
        current_index: usize,
    ) -> Option<usize> {
        let indent =
            self.items.tree_items[current_index].info().indent();

        let mut index = current_index;

        while let Some(selection) = self.selection_updown(index, true)
        {
            index = selection;

            if self.items.tree_items[index].info().indent() < indent {
                break;
            }
        }

        if index == current_index {
            None
        } else {
            Some(index)
        }
    }

    fn selection_left(
        &mut self,
        current_index: usize,
    ) -> Option<usize> {
        let item = &mut self.items.tree_items[current_index];

        if item.kind().is_path() && !item.kind().is_path_collapsed() {
            self.items.collapse(current_index, false);
            return Some(current_index);
        }

        self.select_parent(current_index)
    }

    fn selection_right(
        &mut self,
        current_selection: usize,
    ) -> Option<usize> {
        let item = &mut self.items.tree_items[current_selection];

        if item.kind().is_path() {
            if item.kind().is_path_collapsed() {
                self.items.expand(current_selection);
                return Some(current_selection);
            }
            return self.selection_updown(current_selection, false);
        }

        None
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

        tree.items.collapse(1, false);
        tree.selection = Some(1);

        assert!(tree.move_selection(MoveSelection::Down));

        assert_eq!(tree.selection, Some(3));
    }

    #[test]
    fn test_selection_left_collapse() {
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

        tree.selection = Some(1);

        //collapses 1
        assert!(tree.move_selection(MoveSelection::Left));
        // index will not change
        assert_eq!(tree.selection, Some(1));

        assert!(tree.items.tree_items[1].kind().is_path_collapsed());
        assert!(!tree.items.tree_items[2].info().is_visible());
    }

    #[test]
    fn test_selection_left_parent() {
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

        tree.selection = Some(2);

        assert!(tree.move_selection(MoveSelection::Left));
        assert_eq!(tree.selection, Some(1));

        assert!(tree.move_selection(MoveSelection::Left));
        assert_eq!(tree.selection, Some(1));

        assert!(tree.move_selection(MoveSelection::Left));
        assert_eq!(tree.selection, Some(0));
    }

    #[test]
    fn test_selection_right_expand() {
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

        tree.items.collapse(1, false);
        tree.items.collapse(0, false);
        tree.selection = Some(0);

        assert!(tree.move_selection(MoveSelection::Right));
        assert_eq!(tree.selection, Some(0));
        assert!(!tree.items.tree_items[0].kind().is_path_collapsed());

        assert!(tree.move_selection(MoveSelection::Right));
        assert_eq!(tree.selection, Some(1));
        assert!(tree.items.tree_items[1].kind().is_path_collapsed());

        assert!(tree.move_selection(MoveSelection::Right));
        assert_eq!(tree.selection, Some(1));
        assert!(!tree.items.tree_items[1].kind().is_path_collapsed());
    }

    #[test]
    fn test_selection_top() {
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

        tree.selection = Some(3);

        assert!(tree.move_selection(MoveSelection::Top));
        assert_eq!(tree.selection, Some(0));
    }

    #[test]
    fn test_visible_selection() {
        let items = vec![
            "a/b/c",  //
            "a/b/c2", //
            "a/d",    //
        ];

        //0 a/
        //1   b/
        //2     c
        //3     c2
        //4   d

        let mut tree =
            FileTree::new(&items, &BTreeSet::new()).unwrap();

        tree.items.collapse(1, false);
        tree.selection = Some(4);
        let s = tree.visual_selection();

        assert_eq!(s.count, 3);
        assert_eq!(s.index, 2);
    }
}
