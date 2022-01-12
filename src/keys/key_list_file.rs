use anyhow::Result;
use crossterm::event::KeyEvent;
use ron::{self};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, path::PathBuf};

use super::key_list::KeysList;

#[derive(Serialize, Deserialize, Default)]
pub struct KeysListFile {
	pub tab_status: Option<KeyEvent>,
	pub tab_log: Option<KeyEvent>,
	pub tab_files: Option<KeyEvent>,
	pub tab_stashing: Option<KeyEvent>,
	pub tab_stashes: Option<KeyEvent>,
	pub tab_toggle: Option<KeyEvent>,
	pub tab_toggle_reverse: Option<KeyEvent>,
	pub toggle_workarea: Option<KeyEvent>,
	pub focus_right: Option<KeyEvent>,
	pub focus_left: Option<KeyEvent>,
	pub focus_above: Option<KeyEvent>,
	pub focus_below: Option<KeyEvent>,
	pub exit: Option<KeyEvent>,
	pub quit: Option<KeyEvent>,
	pub exit_popup: Option<KeyEvent>,
	pub open_commit: Option<KeyEvent>,
	pub open_commit_editor: Option<KeyEvent>,
	pub open_help: Option<KeyEvent>,
	pub open_options: Option<KeyEvent>,
	pub move_left: Option<KeyEvent>,
	pub move_right: Option<KeyEvent>,
	pub tree_collapse_recursive: Option<KeyEvent>,
	pub tree_expand_recursive: Option<KeyEvent>,
	pub home: Option<KeyEvent>,
	pub end: Option<KeyEvent>,
	pub move_up: Option<KeyEvent>,
	pub move_down: Option<KeyEvent>,
	pub popup_up: Option<KeyEvent>,
	pub popup_down: Option<KeyEvent>,
	pub page_down: Option<KeyEvent>,
	pub page_up: Option<KeyEvent>,
	pub shift_up: Option<KeyEvent>,
	pub shift_down: Option<KeyEvent>,
	pub enter: Option<KeyEvent>,
	pub blame: Option<KeyEvent>,
	pub edit_file: Option<KeyEvent>,
	pub status_stage_all: Option<KeyEvent>,
	pub status_reset_item: Option<KeyEvent>,
	pub status_ignore_file: Option<KeyEvent>,
	pub diff_stage_lines: Option<KeyEvent>,
	pub diff_reset_lines: Option<KeyEvent>,
	pub stashing_save: Option<KeyEvent>,
	pub stashing_toggle_untracked: Option<KeyEvent>,
	pub stashing_toggle_index: Option<KeyEvent>,
	pub stash_apply: Option<KeyEvent>,
	pub stash_open: Option<KeyEvent>,
	pub stash_drop: Option<KeyEvent>,
	pub cmd_bar_toggle: Option<KeyEvent>,
	pub log_tag_commit: Option<KeyEvent>,
	pub log_mark_commit: Option<KeyEvent>,
	pub commit_amend: Option<KeyEvent>,
	pub copy: Option<KeyEvent>,
	pub create_branch: Option<KeyEvent>,
	pub rename_branch: Option<KeyEvent>,
	pub select_branch: Option<KeyEvent>,
	pub delete_branch: Option<KeyEvent>,
	pub merge_branch: Option<KeyEvent>,
	pub rebase_branch: Option<KeyEvent>,
	pub compare_commits: Option<KeyEvent>,
	pub tags: Option<KeyEvent>,
	pub delete_tag: Option<KeyEvent>,
	pub select_tag: Option<KeyEvent>,
	pub push: Option<KeyEvent>,
	pub open_file_tree: Option<KeyEvent>,
	pub file_find: Option<KeyEvent>,
	pub force_push: Option<KeyEvent>,
	pub pull: Option<KeyEvent>,
	pub abort_merge: Option<KeyEvent>,
	pub undo_commit: Option<KeyEvent>,
	pub stage_unstage_item: Option<KeyEvent>,
	pub tag_annotate: Option<KeyEvent>,
}

impl KeysListFile {
	pub fn read_file(config_file: PathBuf) -> Result<Self> {
		let mut f = File::open(config_file)?;
		let mut buffer = Vec::new();
		f.read_to_end(&mut buffer)?;
		Ok(ron::de::from_bytes(&buffer)?)
	}

	#[rustfmt::skip]
	pub fn get_list(self) -> KeysList {
		let default = KeysList::default();

		KeysList {
			tab_status: self.tab_status.unwrap_or(default.tab_status),
			tab_log: self.tab_log.unwrap_or(default.tab_log),
			tab_files: self.tab_files.unwrap_or(default.tab_files),
			tab_stashing: self.tab_stashing.unwrap_or(default.tab_stashing),
			tab_stashes: self.tab_stashes.unwrap_or(default.tab_stashes),
			tab_toggle: self.tab_toggle.unwrap_or(default.tab_toggle),
			tab_toggle_reverse: self.tab_toggle_reverse.unwrap_or(default.tab_toggle_reverse),
			toggle_workarea: self.toggle_workarea.unwrap_or(default.toggle_workarea),
			focus_right: self.focus_right.unwrap_or(default.focus_right),
			focus_left: self.focus_left.unwrap_or(default.focus_left),
			focus_above: self.focus_above.unwrap_or(default.focus_above),
			focus_below: self.focus_below.unwrap_or(default.focus_below),
			exit: self.exit.unwrap_or(default.exit),
			quit: self.quit.unwrap_or(default.quit),
			exit_popup: self.exit_popup.unwrap_or(default.exit_popup),
			open_commit: self.open_commit.unwrap_or(default.open_commit),
			open_commit_editor: self.open_commit_editor.unwrap_or(default.open_commit_editor),
			open_help: self.open_help.unwrap_or(default.open_help),
			open_options: self.open_options.unwrap_or(default.open_options),
			move_left: self.move_left.unwrap_or(default.move_left),
			move_right: self.move_right.unwrap_or(default.move_right),
			tree_collapse_recursive: self.tree_collapse_recursive.unwrap_or(default.tree_collapse_recursive),
			tree_expand_recursive: self.tree_expand_recursive.unwrap_or(default.tree_expand_recursive),
			home: self.home.unwrap_or(default.home),
			end: self.end.unwrap_or(default.end),
			move_up: self.move_up.unwrap_or(default.move_up),
			move_down: self.move_down.unwrap_or(default.move_down),
			popup_up: self.popup_up.unwrap_or(default.popup_up),
			popup_down: self.popup_down.unwrap_or(default.popup_down),
			page_down: self.page_down.unwrap_or(default.page_down),
			page_up: self.page_up.unwrap_or(default.page_up),
			shift_up: self.shift_up.unwrap_or(default.shift_up),
			shift_down: self.shift_down.unwrap_or(default.shift_down),
			enter: self.enter.unwrap_or(default.enter),
			blame: self.blame.unwrap_or(default.blame),
			edit_file: self.edit_file.unwrap_or(default.edit_file),
			status_stage_all: self.status_stage_all.unwrap_or(default.status_stage_all),
			status_reset_item: self.status_reset_item.unwrap_or(default.status_reset_item),
			status_ignore_file: self.status_ignore_file.unwrap_or(default.status_ignore_file),
			diff_stage_lines: self.diff_stage_lines.unwrap_or(default.diff_stage_lines),
			diff_reset_lines: self.diff_reset_lines.unwrap_or(default.diff_reset_lines),
			stashing_save: self.stashing_save.unwrap_or(default.stashing_save),
			stashing_toggle_untracked: self.stashing_toggle_untracked.unwrap_or(default.stashing_toggle_untracked),
			stashing_toggle_index: self.stashing_toggle_index.unwrap_or(default.stashing_toggle_index),
			stash_apply: self.stash_apply.unwrap_or(default.stash_apply),
			stash_open: self.stash_open.unwrap_or(default.stash_open),
			stash_drop: self.stash_drop.unwrap_or(default.stash_drop),
			cmd_bar_toggle: self.cmd_bar_toggle.unwrap_or(default.cmd_bar_toggle),
			log_tag_commit: self.log_tag_commit.unwrap_or(default.log_tag_commit),
			log_mark_commit: self.log_mark_commit.unwrap_or(default.log_mark_commit),
			commit_amend: self.commit_amend.unwrap_or(default.commit_amend),
			copy: self.copy.unwrap_or(default.copy),
			create_branch: self.create_branch.unwrap_or(default.create_branch),
			rename_branch: self.rename_branch.unwrap_or(default.rename_branch),
			select_branch: self.select_branch.unwrap_or(default.select_branch),
			delete_branch: self.delete_branch.unwrap_or(default.delete_branch),
			merge_branch: self.merge_branch.unwrap_or(default.merge_branch),
			rebase_branch: self.rebase_branch.unwrap_or(default.rebase_branch),
			compare_commits: self.compare_commits.unwrap_or(default.compare_commits),
			tags: self.tags.unwrap_or(default.tags),
			delete_tag: self.delete_tag.unwrap_or(default.delete_tag),
			select_tag: self.select_tag.unwrap_or(default.select_tag),
			push: self.push.unwrap_or(default.push),
			open_file_tree: self.open_file_tree.unwrap_or(default.open_file_tree),
			file_find: self.file_find.unwrap_or(default.file_find),
			force_push: self.force_push.unwrap_or(default.force_push),
			pull: self.pull.unwrap_or(default.pull),
			abort_merge: self.abort_merge.unwrap_or(default.abort_merge),
			undo_commit: self.undo_commit.unwrap_or(default.undo_commit),
			stage_unstage_item: self.stage_unstage_item.unwrap_or(default.stage_unstage_item),
			tag_annotate: self.tag_annotate.unwrap_or(default.tag_annotate),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_load_vim_style_example() {
		assert_eq!(
			KeysListFile::read_file(
				"vim_style_key_config.ron".into()
			)
			.is_ok(),
			true
		);
	}
}
