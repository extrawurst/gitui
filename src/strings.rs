use crate::keys::{get_hint, SharedKeyConfig};

pub mod order {
    pub static NAV: i8 = 1;
}

pub static PUSH_POPUP_MSG: &str = "Push";
pub static PUSH_POPUP_PROGRESS_NONE: &str = "preparing...";
pub static PUSH_POPUP_STATES_ADDING: &str = "adding objects (1/3)";
pub static PUSH_POPUP_STATES_DELTAS: &str = "deltas (2/3)";
pub static PUSH_POPUP_STATES_PUSHING: &str = "pushing (3/3)";

pub static SELECT_BRANCH_POPUP_MSG: &str = "Switch Branch";

pub fn title_status(key_config: &SharedKeyConfig) -> String {
    format!(
        "Unstaged Changes [{}]",
        get_hint(key_config.focus_workdir)
    )
}
pub fn title_diff(_key_config: &SharedKeyConfig) -> String {
    "Diff: ".to_string()
}
pub fn title_index(key_config: &SharedKeyConfig) -> String {
    format!("Staged Changes [{}]", get_hint(key_config.focus_stage))
}
pub fn tab_status(key_config: &SharedKeyConfig) -> String {
    format!("Status [{}]", get_hint(key_config.tab_status))
}
pub fn tab_log(key_config: &SharedKeyConfig) -> String {
    format!("Log [{}]", get_hint(key_config.tab_log))
}
pub fn tab_stashing(key_config: &SharedKeyConfig) -> String {
    format!("Stashing [{}]", get_hint(key_config.tab_stashing))
}
pub fn tab_stashes(key_config: &SharedKeyConfig) -> String {
    format!("Stashes [{}]", get_hint(key_config.tab_stashes))
}
pub fn tab_divider(_key_config: &SharedKeyConfig) -> String {
    " | ".to_string()
}
pub fn cmd_splitter(_key_config: &SharedKeyConfig) -> String {
    " ".to_string()
}
pub fn msg_opening_editor(_key_config: &SharedKeyConfig) -> String {
    "opening editor...".to_string()
}
pub fn msg_title_error(_key_config: &SharedKeyConfig) -> String {
    "Error".to_string()
}
pub fn commit_title(_key_config: &SharedKeyConfig) -> String {
    "Commit".to_string()
}
pub fn commit_title_amend(_key_config: &SharedKeyConfig) -> String {
    "Commit (Amend)".to_string()
}
pub fn commit_msg(_key_config: &SharedKeyConfig) -> String {
    "type commit message..".to_string()
}
pub fn commit_editor_msg(_key_config: &SharedKeyConfig) -> String {
    r##"
# Edit your commit message
# Lines starting with '#' will be ignored"##
        .to_string()
}
pub fn stash_popup_title(_key_config: &SharedKeyConfig) -> String {
    "Stash".to_string()
}
pub fn stash_popup_msg(_key_config: &SharedKeyConfig) -> String {
    "type name (optional)".to_string()
}
pub fn confirm_title_reset(_key_config: &SharedKeyConfig) -> String {
    "Reset".to_string()
}
pub fn confirm_title_stashdrop(
    _key_config: &SharedKeyConfig,
) -> String {
    "Drop".to_string()
}
pub fn confirm_msg_reset(_key_config: &SharedKeyConfig) -> String {
    "confirm file reset?".to_string()
}
pub fn confirm_msg_stashdrop(
    _key_config: &SharedKeyConfig,
) -> String {
    "confirm stash drop?".to_string()
}
pub fn confirm_msg_resethunk(
    _key_config: &SharedKeyConfig,
) -> String {
    "confirm reset hunk?".to_string()
}
pub fn confirm_title_delete_branch(
    _key_config: &SharedKeyConfig,
) -> String {
    "Delete Branch".to_string()
}
pub fn confirm_msg_delete_branch(
    _key_config: &SharedKeyConfig,
    branch_ref: &str,
) -> String {
    format!("Confirm deleting branch: '{}' ?", branch_ref)
}
pub fn log_title(_key_config: &SharedKeyConfig) -> String {
    "Commit".to_string()
}
pub fn tag_commit_popup_title(
    _key_config: &SharedKeyConfig,
) -> String {
    "Tag".to_string()
}
pub fn tag_commit_popup_msg(_key_config: &SharedKeyConfig) -> String {
    "type tag".to_string()
}
pub fn stashlist_title(_key_config: &SharedKeyConfig) -> String {
    "Stashes".to_string()
}
pub fn help_title(_key_config: &SharedKeyConfig) -> String {
    "Help: all commands".to_string()
}
pub fn stashing_files_title(_key_config: &SharedKeyConfig) -> String {
    "Files to Stash".to_string()
}
pub fn stashing_options_title(
    _key_config: &SharedKeyConfig,
) -> String {
    "Options".to_string()
}
pub fn loading_text(_key_config: &SharedKeyConfig) -> String {
    "Loading ...".to_string()
}
pub fn create_branch_popup_title(
    _key_config: &SharedKeyConfig,
) -> String {
    "Branch".to_string()
}
pub fn create_branch_popup_msg(
    _key_config: &SharedKeyConfig,
) -> String {
    "type branch name".to_string()
}

pub fn rename_branch_popup_title(
    _key_config: &SharedKeyConfig,
) -> String {
    "Rename Branch".to_string()
}
pub fn rename_branch_popup_msg(
    _key_config: &SharedKeyConfig,
) -> String {
    "new branch name".to_string()
}

pub mod commit {
    use crate::keys::SharedKeyConfig;
    pub fn details_author(_key_config: &SharedKeyConfig) -> String {
        "Author: ".to_string()
    }
    pub fn details_committer(
        _key_config: &SharedKeyConfig,
    ) -> String {
        "Committer: ".to_string()
    }
    pub fn details_sha(_key_config: &SharedKeyConfig) -> String {
        "Sha: ".to_string()
    }
    pub fn details_date(_key_config: &SharedKeyConfig) -> String {
        "Date: ".to_string()
    }
    pub fn details_tags(_key_config: &SharedKeyConfig) -> String {
        "Tags: ".to_string()
    }
    pub fn details_info_title(
        _key_config: &SharedKeyConfig,
    ) -> String {
        "Info".to_string()
    }
    pub fn details_message_title(
        _key_config: &SharedKeyConfig,
    ) -> String {
        "Message".to_string()
    }
    pub fn details_files_title(
        _key_config: &SharedKeyConfig,
    ) -> String {
        "Files:".to_string()
    }
}

pub mod commands {
    use crate::components::CommandText;
    use crate::keys::{get_hint, SharedKeyConfig};

    static CMD_GROUP_GENERAL: &str = "-- General --";
    static CMD_GROUP_DIFF: &str = "-- Diff --";
    static CMD_GROUP_CHANGES: &str = "-- Changes --";
    static CMD_GROUP_COMMIT: &str = "-- Commit --";
    static CMD_GROUP_STASHING: &str = "-- Stashing --";
    static CMD_GROUP_STASHES: &str = "-- Stashes --";
    static CMD_GROUP_LOG: &str = "-- Log --";

    pub fn toggle_tabs(key_config: &SharedKeyConfig) -> CommandText {
        CommandText::new(
            format!("Next [{}]", get_hint(key_config.tab_toggle)),
            "switch to next tab",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn toggle_tabs_direct(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!(
                "Tab [{}{}{}{}]",
                get_hint(key_config.tab_status),
                get_hint(key_config.tab_log),
                get_hint(key_config.tab_stashing),
                get_hint(key_config.tab_stashes),
            ),
            "switch top level tabs directly",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn help_open(key_config: &SharedKeyConfig) -> CommandText {
        CommandText::new(
            format!("Help [{}]", get_hint(key_config.open_help)),
            "open this help screen",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn navigate_commit_message(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!(
                "Nav [{}{}]",
                get_hint(key_config.move_up),
                get_hint(key_config.move_down)
            ),
            "navigate commit message",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn navigate_tree(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!(
                "Nav [{}{}{}{}]",
                get_hint(key_config.move_up),
                get_hint(key_config.move_down),
                get_hint(key_config.move_right),
                get_hint(key_config.move_left)
            ),
            "navigate tree view",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn scroll(key_config: &SharedKeyConfig) -> CommandText {
        CommandText::new(
            format!(
                "Scroll [{}{}]",
                get_hint(key_config.focus_above),
                get_hint(key_config.focus_below)
            ),
            "scroll up or down in focused view",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn copy(key_config: &SharedKeyConfig) -> CommandText {
        CommandText::new(
            format!("Copy [{}]", get_hint(key_config.copy),),
            "copy selected lines to clipboard",
            CMD_GROUP_DIFF,
        )
    }
    pub fn diff_home_end(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!(
                "Jump up/down [{},{},{},{}]",
                get_hint(key_config.home),
                get_hint(key_config.end),
                get_hint(key_config.move_up),
                get_hint(key_config.move_down)
            ),
            "scroll to top or bottom of diff",
            CMD_GROUP_DIFF,
        )
    }
    pub fn diff_hunk_add(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!("Add hunk [{}]", get_hint(key_config.enter),),
            "adds selected hunk to stage",
            CMD_GROUP_DIFF,
        )
    }
    pub fn diff_hunk_revert(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!(
                "Revert hunk [{}]",
                get_hint(key_config.status_reset_item),
            ),
            "reverts selected hunk",
            CMD_GROUP_DIFF,
        )
    }
    pub fn diff_hunk_remove(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!("Remove hunk [{}]", get_hint(key_config.enter),),
            "removes selected hunk from stage",
            CMD_GROUP_DIFF,
        )
    }
    pub fn close_popup(key_config: &SharedKeyConfig) -> CommandText {
        CommandText::new(
            format!("Close [{}]", get_hint(key_config.exit_popup),),
            "close overlay (e.g commit, help)",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn close_msg(key_config: &SharedKeyConfig) -> CommandText {
        CommandText::new(
            format!("Close [{}]", get_hint(key_config.enter),),
            "close msg popup (e.g msg)",
            CMD_GROUP_GENERAL,
        )
        .hide_help()
    }
    pub fn select_staging(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!(
                "To stage [{}]",
                get_hint(key_config.focus_stage),
            ),
            "focus/select staging area",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn select_status(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!(
                "To files [{},{}]",
                get_hint(key_config.tab_status),
                get_hint(key_config.tab_log),
            ),
            "focus/select file tree of staged or unstaged files",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn select_unstaged(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!(
                "To unstaged [{}]",
                get_hint(key_config.focus_workdir),
            ),
            "focus/select unstaged area",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn commit_open(key_config: &SharedKeyConfig) -> CommandText {
        CommandText::new(
            format!("Commit [{}]", get_hint(key_config.open_commit),),
            "open commit popup (available in non-empty stage)",
            CMD_GROUP_COMMIT,
        )
    }
    pub fn commit_open_editor(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!(
                "Open editor [{}]",
                get_hint(key_config.open_commit_editor),
            ),
            "open commit editor (available in non-empty stage)",
            CMD_GROUP_COMMIT,
        )
    }
    pub fn commit_enter(key_config: &SharedKeyConfig) -> CommandText {
        CommandText::new(
            format!("Commit [{}]", get_hint(key_config.enter),),
            "commit (available when commit message is non-empty)",
            CMD_GROUP_COMMIT,
        )
    }
    pub fn commit_amend(key_config: &SharedKeyConfig) -> CommandText {
        CommandText::new(
            format!("Amend [{}]", get_hint(key_config.commit_amend),),
            "amend last commit",
            CMD_GROUP_COMMIT,
        )
    }
    pub fn edit_item(key_config: &SharedKeyConfig) -> CommandText {
        CommandText::new(
            format!("Edit Item [{}]", get_hint(key_config.edit_file),),
            "edit the currently selected file in an external editor",
            CMD_GROUP_CHANGES,
        )
    }
    pub fn stage_item(key_config: &SharedKeyConfig) -> CommandText {
        CommandText::new(
            format!("Stage Item [{}]", get_hint(key_config.enter),),
            "stage currently selected file or entire path",
            CMD_GROUP_CHANGES,
        )
    }
    pub fn stage_all(key_config: &SharedKeyConfig) -> CommandText {
        CommandText::new(
            format!(
                "Stage All [{}]",
                get_hint(key_config.status_stage_all),
            ),
            "stage all changes (in unstaged files)",
            CMD_GROUP_CHANGES,
        )
    }
    pub fn unstage_item(key_config: &SharedKeyConfig) -> CommandText {
        CommandText::new(
            format!("Unstage Item [{}]", get_hint(key_config.enter),),
            "unstage currently selected file or entire path",
            CMD_GROUP_CHANGES,
        )
    }
    pub fn unstage_all(key_config: &SharedKeyConfig) -> CommandText {
        CommandText::new(
            format!(
                "Unstage all [{}]",
                get_hint(key_config.status_stage_all),
            ),
            "unstage all files (in staged files)",
            CMD_GROUP_CHANGES,
        )
    }
    pub fn reset_item(key_config: &SharedKeyConfig) -> CommandText {
        CommandText::new(
            format!(
                "Reset Item [{}]",
                get_hint(key_config.stash_drop),
            ),
            "revert changes in selected file or entire path",
            CMD_GROUP_CHANGES,
        )
    }
    pub fn ignore_item(key_config: &SharedKeyConfig) -> CommandText {
        CommandText::new(
            format!(
                "Ignore [{}]",
                get_hint(key_config.status_ignore_file),
            ),
            "Add file or path to .gitignore",
            CMD_GROUP_CHANGES,
        )
    }

    pub fn diff_focus_left(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!("Back [{}]", get_hint(key_config.focus_left),),
            "view and select changed files",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn diff_focus_right(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!("Diff [{}]", get_hint(key_config.focus_right),),
            "inspect file diff",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn quit(key_config: &SharedKeyConfig) -> CommandText {
        CommandText::new(
            format!("Quit [{}]", get_hint(key_config.exit),),
            "quit gitui application",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn reset_confirm(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!("Confirm [{}]", get_hint(key_config.enter),),
            "resets the file in question",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn stashing_save(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!("Save [{}]", get_hint(key_config.stashing_save),),
            "opens stash name input popup",
            CMD_GROUP_STASHING,
        )
    }
    pub fn stashing_toggle_indexed(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!(
                "Toggle Staged [{}]",
                get_hint(key_config.stashing_toggle_index),
            ),
            "toggle including staged files into stash",
            CMD_GROUP_STASHING,
        )
    }
    pub fn stashing_toggle_untracked(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!(
                "Toggle Untracked [{}]",
                get_hint(key_config.stashing_toggle_untracked),
            ),
            "toggle including untracked files into stash",
            CMD_GROUP_STASHING,
        )
    }
    pub fn stashing_confirm_msg(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!("Stash [{}]", get_hint(key_config.enter),),
            "save files to stash",
            CMD_GROUP_STASHING,
        )
    }
    pub fn stashlist_apply(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!("Apply [{}]", get_hint(key_config.enter),),
            "apply selected stash",
            CMD_GROUP_STASHES,
        )
    }
    pub fn stashlist_drop(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!("Drop [{}]", get_hint(key_config.stash_drop),),
            "drop selected stash",
            CMD_GROUP_STASHES,
        )
    }
    pub fn stashlist_inspect(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!("Inspect [{}]", get_hint(key_config.focus_right),),
            "open stash commit details (allows to diff files)",
            CMD_GROUP_STASHES,
        )
    }
    pub fn log_details_toggle(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!("Details [{}]", get_hint(key_config.enter),),
            "open details of selected commit",
            CMD_GROUP_LOG,
        )
    }
    pub fn log_details_open(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!("Inspect [{}]", get_hint(key_config.focus_right),),
            "inspect selected commit in detail",
            CMD_GROUP_LOG,
        )
    }
    pub fn log_tag_commit(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!("Tag [{}]", get_hint(key_config.log_tag_commit),),
            "tag commit",
            CMD_GROUP_LOG,
        )
    }
    pub fn tag_commit_confirm_msg(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!("Tag [{}]", get_hint(key_config.enter),),
            "tag commit",
            CMD_GROUP_LOG,
        )
    }
    pub fn create_branch_confirm_msg(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!("Create Branch [{}]", get_hint(key_config.enter),),
            "create branch",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn open_branch_create_popup(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!(
                "Create [{}]",
                get_hint(key_config.create_branch),
            ),
            "open create branch popup",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn rename_branch_confirm_msg(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!("Rename Branch [{}]", get_hint(key_config.enter),),
            "rename branch",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn rename_branch_popup(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!(
                "Rename Branch [{}]",
                get_hint(key_config.rename_branch),
            ),
            "rename branch",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn delete_branch_popup(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!(
                "Delete [{}]",
                get_hint(key_config.delete_branch),
            ),
            "delete a branch",
            CMD_GROUP_GENERAL,
        )
    }
    pub fn open_branch_select_popup(
        key_config: &SharedKeyConfig,
    ) -> CommandText {
        CommandText::new(
            format!(
                "Branches [{}]",
                get_hint(key_config.select_branch),
            ),
            "open select branch popup",
            CMD_GROUP_GENERAL,
        )
    }

    pub fn status_push(key_config: &SharedKeyConfig) -> CommandText {
        CommandText::new(
            format!("Push [{}]", get_hint(key_config.push),),
            "push to origin",
            CMD_GROUP_GENERAL,
        )
    }
}
