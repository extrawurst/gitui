use anyhow::Result;
use asyncgit::{StatusItem, StatusItemType};
use std::{
	collections::BTreeSet,
	ops::{Index, IndexMut},
	path::Path,
};

use filetreelist::{FileTreeItem, TreeItemInfo};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Item {
	tree_item: FileTreeItem,
	status_item_type: Option<StatusItemType>,
}

impl Item {
	pub fn collapse_path(&mut self) {
		self.tree_item.collapse_path();
	}

	pub fn expand_path(&mut self) {
		self.tree_item.expand_path();
	}

	pub fn info(&self) -> &TreeItemInfo {
		self.tree_item.info()
	}

	pub fn is_path(&self) -> bool {
		self.tree_item.kind().is_path()
	}

	pub fn is_path_collapsed(&self) -> bool {
		self.tree_item.kind().is_path_collapsed()
	}

	pub fn is_visible(&self) -> bool {
		self.tree_item.info().is_visible()
	}

	pub fn status(&self) -> Option<StatusItem> {
		self.status_item_type.map(|s| StatusItem {
			path: self.info().full_path_str().to_string(),
			status: s,
		})
	}

	pub fn set_visible(&mut self, visible: bool) {
		self.tree_item.info_mut().set_visible(visible);
	}
}

///
#[derive(Default)]
pub struct FileTreeItems {
	items: Vec<Item>,
	file_count: usize,
}

impl FileTreeItems {
	///
	pub(crate) fn new(
		list: &[StatusItem],
		collapsed: &BTreeSet<&str>,
	) -> Result<Self> {
		let mut items = Vec::with_capacity(list.len());
		let mut paths_added = BTreeSet::new();

		for e in list {
			let item_path = Path::new(&e.path);

			Self::push_dirs(
				item_path,
				&mut items,
				&mut paths_added,
				collapsed,
			)?;
			let tree_item = FileTreeItem::new_file(item_path)?;
			let status_item_type = Some(e.status);
			items.push(Item {
				tree_item,
				status_item_type,
			});
		}

		Ok(Self {
			items,
			file_count: list.len(),
		})
	}

	pub(crate) fn index_tree_item(
		&self,
		idx: usize,
	) -> &FileTreeItem {
		&self.items[idx].tree_item
	}

	///
	pub(crate) const fn items(&self) -> &Vec<Item> {
		&self.items
	}

	///
	pub(crate) fn len(&self) -> usize {
		self.items.len()
	}

	///
	pub const fn file_count(&self) -> usize {
		self.file_count
	}

	///
	pub(crate) fn find_parent_index(&self, index: usize) -> usize {
		let item_indent = &self.items[index].info().indent();
		let mut parent_index = index;
		while item_indent <= &self.items[parent_index].info().indent()
		{
			if parent_index == 0 {
				return 0;
			}
			parent_index -= 1;
		}

		parent_index
	}

	fn push_dirs<'a>(
		item_path: &'a Path,
		nodes: &mut Vec<Item>,
		paths_added: &mut BTreeSet<&'a Path>,
		collapsed: &BTreeSet<&str>,
	) -> Result<()> {
		let mut ancestors =
			{ item_path.ancestors().skip(1).collect::<Vec<_>>() };
		ancestors.reverse();

		for c in &ancestors {
			if c.parent().is_some() && !paths_added.contains(c) {
				paths_added.insert(c);
				//TODO: get rid of expect
				let path_string =
					String::from(c.to_str().expect("invalid path"));
				let is_collapsed =
					collapsed.contains(path_string.as_str());

				let tree_item =
					FileTreeItem::new_path(c, is_collapsed)?;
				nodes.push(Item {
					tree_item,
					status_item_type: None,
				});
			}
		}

		Ok(())
	}

	pub fn multiple_items_at_path(&self, index: usize) -> bool {
		let tree_items = self.items();
		let mut idx_temp_inner;
		if index + 2 < tree_items.len() {
			idx_temp_inner = index + 1;
			while idx_temp_inner < tree_items.len().saturating_sub(1)
				&& tree_items[index].info().indent()
					< tree_items[idx_temp_inner].info().indent()
			{
				idx_temp_inner += 1;
			}
		} else {
			return false;
		}

		tree_items[idx_temp_inner].info().indent()
			== tree_items[index].info().indent()
	}
}

impl IndexMut<usize> for FileTreeItems {
	fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
		&mut self.items[idx]
	}
}

impl Index<usize> for FileTreeItems {
	type Output = Item;

	fn index(&self, idx: usize) -> &Self::Output {
		&self.items[idx]
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use asyncgit::StatusItemType;

	fn string_vec_to_status(items: &[&str]) -> Vec<StatusItem> {
		items
			.iter()
			.map(|a| StatusItem {
				path: String::from(*a),
				status: StatusItemType::Modified,
			})
			.collect::<Vec<_>>()
	}

	#[test]
	fn test_simple() {
		let items = string_vec_to_status(&[
			"file.txt", //
		]);

		let res =
			FileTreeItems::new(&items, &BTreeSet::new()).unwrap();

		let expected_tree_item =
			FileTreeItem::new_file(Path::new(&items[0].path))
				.unwrap();
		assert_eq!(
			res.items,
			vec![Item {
				tree_item: expected_tree_item,
				status_item_type: Some(items[0].status),
			}]
		);

		let items = string_vec_to_status(&[
			"file.txt",  //
			"file2.txt", //
		]);

		let res =
			FileTreeItems::new(&items, &BTreeSet::new()).unwrap();

		assert_eq!(res.items.len(), 2);
		assert_eq!(
			res.items[1].info().path_str().to_string(),
			items[1].path
		);
	}

	#[test]
	fn test_folder() {
		let items = string_vec_to_status(&[
			"a/file.txt", //
		]);

		let res = FileTreeItems::new(&items, &BTreeSet::new())
			.unwrap()
			.items
			.iter()
			.map(|i| i.info().full_path_str().to_string())
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

		let list =
			FileTreeItems::new(&items, &BTreeSet::new()).unwrap();
		let mut res = list
			.items
			.iter()
			.map(|i| (i.info().indent(), i.info().path_str()));

		assert_eq!(res.next(), Some((0, "a")));
		assert_eq!(res.next(), Some((1, "b")));
		assert_eq!(res.next(), Some((2, "file.txt")));
	}

	#[test]
	fn test_indent_folder_file_name() {
		let items = string_vec_to_status(&[
			"a/b",   //
			"a.txt", //
		]);

		let list =
			FileTreeItems::new(&items, &BTreeSet::new()).unwrap();
		let mut res = list
			.items
			.iter()
			.map(|i| (i.info().indent(), i.info().path_str()));

		assert_eq!(res.next(), Some((0, "a")));
		assert_eq!(res.next(), Some((1, "b")));
		assert_eq!(res.next(), Some((0, "a.txt")));
	}

	#[test]
	fn test_folder_dup() {
		let items = string_vec_to_status(&[
			"a/file.txt",  //
			"a/file2.txt", //
		]);

		let res = FileTreeItems::new(&items, &BTreeSet::new())
			.unwrap()
			.items
			.iter()
			.map(|i| i.info().full_path_str().to_string())
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

	#[test]
	fn test_multiple_items_at_path() {
		//0 a/
		//1   b/
		//2     c/
		//3       d
		//4     e/
		//5       f

		let res = FileTreeItems::new(
			&string_vec_to_status(&[
				"a/b/c/d", //
				"a/b/e/f", //
			]),
			&BTreeSet::new(),
		)
		.unwrap();

		assert_eq!(res.multiple_items_at_path(0), false);
		assert_eq!(res.multiple_items_at_path(1), false);
		assert_eq!(res.multiple_items_at_path(2), true);
	}

	#[test]
	fn test_find_parent() {
		//0 a/
		//1   b/
		//2     c
		//3     d

		let res = FileTreeItems::new(
			&string_vec_to_status(&[
				"a/b/c", //
				"a/b/d", //
			]),
			&BTreeSet::new(),
		)
		.unwrap();

		assert_eq!(res.find_parent_index(3), 1);
	}
}
