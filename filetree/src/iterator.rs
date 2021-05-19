use crate::FileTree;

pub struct TreeIterator<'a> {
    tree: &'a FileTree,
    index: usize,
    increments: Option<usize>,
    amount: usize,
}

impl<'a> TreeIterator<'a> {
    pub const fn new(
        tree: &'a FileTree,
        start: usize,
        amount: usize,
    ) -> Self {
        TreeIterator {
            amount,
            increments: None,
            index: start,
            tree,
        }
    }
}

impl<'a> Iterator for TreeIterator<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.increments.unwrap_or_default() < self.amount {
            let mut init = self.increments.is_none();

            if let Some(i) = self.increments.as_mut() {
                *i += 1;
            } else {
                self.increments = Some(0);
            };

            loop {
                if !init {
                    self.index += 1;
                }
                init = false;

                if self.index >= self.tree.len() {
                    break;
                }

                let elem = &self.tree.items[self.index];

                if elem.info().is_visible() {
                    return Some(self.index);
                }
            }
        }

        None
    }
}
