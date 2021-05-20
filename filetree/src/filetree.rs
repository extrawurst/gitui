use crate::{
    error::Result, filetreeitems::FileTreeItems,
    tree_iter::TreeIterator,
};
use std::collections::BTreeSet;

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
}
