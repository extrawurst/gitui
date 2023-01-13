use anyhow::Result;
use ron::{self};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, path::PathBuf};

use super::key_list::{GituiKeyEvent, KeysList};

#[derive(Serialize, Deserialize, Default)]
pub struct KeysListFile {
	pub tab_status: Option<GituiKeyEvent>,
	pub tab_log: Option<GituiKeyEvent>,
	pub tab_files: Option<GituiKeyEvent>,
	pub tab_stashing: Option<GituiKeyEvent>,
	pub tab_stashes: Option<GituiKeyEvent>,
	pub tab_toggle: Option<GituiKeyEvent>,
	pub tab_toggle_reverse: Option<GituiKeyEvent>,
	pub toggle_workarea: Option<GituiKeyEvent>,
	pub focus_right: Option<GituiKeyEvent>,
	pub focus_left: Option<GituiKeyEvent>,
	pub focus_above: Option<GituiKeyEvent>,
	pub focus_below: Option<GituiKeyEvent>,
	pub exit: Option<GituiKeyEvent>,
	pub quit: Option<GituiKeyEvent>,
	pub exit_popup: Option<GituiKeyEvent>,
	pub open_commit: Option<GituiKeyEvent>,
	pub open_commit_editor: Option<GituiKeyEvent>,
	pub open_help: Option<GituiKeyEvent>,
	pub open_options: Option<GituiKeyEvent>,
	pub move_left: Option<GituiKeyEvent>,
	pub move_right: Option<GituiKeyEvent>,
	pub tree_collapse_recursive: Option<GituiKeyEvent>,
	pub tree_expand_recursive: Option<GituiKeyEvent>,
	pub home: Option<GituiKeyEvent>,
	pub end: Option<GituiKeyEvent>,
	pub move_up: Option<GituiKeyEvent>,
	pub move_down: Option<GituiKeyEvent>,
	pub popup_up: Option<GituiKeyEvent>,
	pub popup_down: Option<GituiKeyEvent>,
	pub page_down: Option<GituiKeyEvent>,
	pub page_up: Option<GituiKeyEvent>,
	pub shift_up: Option<GituiKeyEvent>,
	pub shift_down: Option<GituiKeyEvent>,
	pub enter: Option<GituiKeyEvent>,
	pub blame: Option<GituiKeyEvent>,
	pub edit_file: Option<GituiKeyEvent>,
	pub file_history: Option<GituiKeyEvent>,
	pub status_stage_all: Option<GituiKeyEvent>,
	pub status_reset_item: Option<GituiKeyEvent>,
	pub status_ignore_file: Option<GituiKeyEvent>,
	pub diff_stage_lines: Option<GituiKeyEvent>,
	pub diff_reset_lines: Option<GituiKeyEvent>,
	pub stashing_save: Option<GituiKeyEvent>,
	pub stashing_toggle_untracked: Option<GituiKeyEvent>,
	pub stashing_toggle_index: Option<GituiKeyEvent>,
	pub stash_apply: Option<GituiKeyEvent>,
	pub stash_open: Option<GituiKeyEvent>,
	pub stash_drop: Option<GituiKeyEvent>,
	pub cmd_bar_toggle: Option<GituiKeyEvent>,
	pub log_tag_commit: Option<GituiKeyEvent>,
	pub log_mark_commit: Option<GituiKeyEvent>,
	pub commit_amend: Option<GituiKeyEvent>,
	pub toggle_verify: Option<GituiKeyEvent>,
	pub copy: Option<GituiKeyEvent>,
	pub create_branch: Option<GituiKeyEvent>,
	pub rename_branch: Option<GituiKeyEvent>,
	pub select_branch: Option<GituiKeyEvent>,
	pub delete_branch: Option<GituiKeyEvent>,
	pub merge_branch: Option<GituiKeyEvent>,
	pub rebase_branch: Option<GituiKeyEvent>,
	pub compare_commits: Option<GituiKeyEvent>,
	pub tags: Option<GituiKeyEvent>,
	pub delete_tag: Option<GituiKeyEvent>,
	pub select_tag: Option<GituiKeyEvent>,
	pub push: Option<GituiKeyEvent>,
	pub open_file_tree: Option<GituiKeyEvent>,
	pub file_find: Option<GituiKeyEvent>,
	pub force_push: Option<GituiKeyEvent>,
	pub fetch: Option<GituiKeyEvent>,
	pub pull: Option<GituiKeyEvent>,
	pub abort_merge: Option<GituiKeyEvent>,
	pub undo_commit: Option<GituiKeyEvent>,
	pub stage_unstage_item: Option<GituiKeyEvent>,
	pub tag_annotate: Option<GituiKeyEvent>,
	pub view_submodules: Option<GituiKeyEvent>,
	pub view_submodule_parent: Option<GituiKeyEvent>,
	pub update_dubmodule: Option<GituiKeyEvent>,
	pub commit_history_next: Option<GituiKeyEvent>,
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
			file_history: self.file_history.unwrap_or(default.file_history),
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
			toggle_verify: self.toggle_verify.unwrap_or(default.toggle_verify),
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
			fetch: self.fetch.unwrap_or(default.fetch),
			pull: self.pull.unwrap_or(default.pull),
			abort_merge: self.abort_merge.unwrap_or(default.abort_merge),
			undo_commit: self.undo_commit.unwrap_or(default.undo_commit),
			stage_unstage_item: self.stage_unstage_item.unwrap_or(default.stage_unstage_item),
			tag_annotate: self.tag_annotate.unwrap_or(default.tag_annotate),
			view_submodules: self.view_submodules.unwrap_or(default.view_submodules),
			view_submodule_parent: self.view_submodule_parent.unwrap_or(default.view_submodule_parent),
			update_submodule: self.update_dubmodule.unwrap_or(default.update_submodule),
			commit_history_next: self.commit_history_next.unwrap_or(default.commit_history_next),
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
