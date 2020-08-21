pub static TITLE_STATUS: &str = "Unstaged Changes [w]";
pub static TITLE_DIFF: &str = "Diff: ";
pub static TITLE_INDEX: &str = "Staged Changes [s]";

pub static TAB_STATUS: &str = "Status [1]";
pub static TAB_LOG: &str = "Log [2]";
pub static TAB_STASHING: &str = "Stashing [3]";
pub static TAB_STASHES: &str = "Stashes [4]";
pub static TAB_DIVIDER: &str = " | ";

pub static CMD_SPLITTER: &str = " ";

pub static MSG_OPENING_EDITOR: &str = "opening editor...";
pub static MSG_TITLE_ERROR: &str = "Error";
pub static COMMIT_TITLE: &str = "Commit";
pub static COMMIT_TITLE_AMEND: &str = "Commit (Amend)";
pub static COMMIT_MSG: &str = "type commit message..";
pub static COMMIT_EDITOR_MSG: &str = r##"
# Edit your commit message
# Lines starting with '#' will be ignored"##;
pub static STASH_POPUP_TITLE: &str = "Stash";
pub static STASH_POPUP_MSG: &str = "type name (optional)";
pub static CONFIRM_TITLE_RESET: &str = "Reset";
pub static CONFIRM_TITLE_STASHDROP: &str = "Drop";
pub static CONFIRM_MSG_RESET: &str = "confirm file reset?";
pub static CONFIRM_MSG_STASHDROP: &str = "confirm stash drop?";
pub static CONFIRM_MSG_RESETHUNK: &str = "confirm reset hunk?";

pub static LOG_TITLE: &str = "Commit";

pub static TAG_COMMIT_POPUP_TITLE: &str = "Tag";
pub static TAG_COMMIT_POPUP_MSG: &str = "type tag";

pub static STASHLIST_TITLE: &str = "Stashes";

pub static HELP_TITLE: &str = "Help: all commands";

pub static STASHING_FILES_TITLE: &str = "Files to Stash";
pub static STASHING_OPTIONS_TITLE: &str = "Options";

pub static LOADING_TEXT: &str = "Loading ...";

pub mod commit {
    pub static DETAILS_AUTHOR: &str = "Author: ";
    pub static DETAILS_COMMITTER: &str = "Committer: ";
    pub static DETAILS_SHA: &str = "SHA: ";
    pub static DETAILS_DATE: &str = "Date: ";
    pub static DETAILS_TAGS: &str = "Tags: ";

    pub static DETAILS_INFO_TITLE: &str = "Info";
    pub static DETAILS_MESSAGE_TITLE: &str = "Message";
    pub static DETAILS_FILES_TITLE: &str = "Files:";
}

pub mod order {
    pub static NAV: i8 = 1;
}

pub mod commands {
    use crate::components::CommandText;

    static CMD_GROUP_GENERAL: &str = "-- General --";
    static CMD_GROUP_DIFF: &str = "-- Diff --";
    static CMD_GROUP_CHANGES: &str = "-- Changes --";
    static CMD_GROUP_COMMIT: &str = "-- Commit --";
    static CMD_GROUP_STASHING: &str = "-- Stashing --";
    static CMD_GROUP_STASHES: &str = "-- Stashes --";
    static CMD_GROUP_LOG: &str = "-- Log --";

    ///
    pub static TOGGLE_TABS: CommandText = CommandText::new(
        "Next [tab]",
        "switch to next tab",
        CMD_GROUP_GENERAL,
    );
    ///
    pub static TOGGLE_TABS_DIRECT: CommandText = CommandText::new(
        "Tab [1234]",
        "switch top level tabs directly",
        CMD_GROUP_GENERAL,
    );
    ///
    pub static HELP_OPEN: CommandText = CommandText::new(
        "Help [h]",
        "open this help screen",
        CMD_GROUP_GENERAL,
    );
    ///
    pub static NAVIGATE_COMMIT_MESSAGE: CommandText =
        CommandText::new(
            "Nav [\u{2191}\u{2193}]",
            "navigate commit message",
            CMD_GROUP_GENERAL,
        );
    ///
    pub static NAVIGATE_TREE: CommandText = CommandText::new(
        "Nav [\u{2190}\u{2191}\u{2192}\u{2193}]",
        "navigate tree view",
        CMD_GROUP_GENERAL,
    );
    ///
    pub static SCROLL: CommandText = CommandText::new(
        "Scroll [\u{2191}\u{2193}]",
        "scroll up or down in focused view",
        CMD_GROUP_GENERAL,
    );
    ///
    pub static COPY: CommandText = CommandText::new(
        "Copy [y]",
        "copy selected lines to clipboard",
        CMD_GROUP_DIFF,
    );
    ///
    pub static DIFF_HOME_END: CommandText = CommandText::new(
        "Jump up/down [home,end,\u{2191} up,\u{2193} down]",
        "scroll to top or bottom of diff",
        CMD_GROUP_DIFF,
    );
    ///
    pub static DIFF_HUNK_ADD: CommandText = CommandText::new(
        "Add hunk [enter]",
        "adds selected hunk to stage",
        CMD_GROUP_DIFF,
    );
    ///
    pub static DIFF_HUNK_REVERT: CommandText = CommandText::new(
        "Revert hunk [D]",
        "reverts selected hunk",
        CMD_GROUP_DIFF,
    );
    ///
    pub static DIFF_HUNK_REMOVE: CommandText = CommandText::new(
        "Remove hunk [enter]",
        "removes selected hunk from stage",
        CMD_GROUP_DIFF,
    );
    ///
    pub static CLOSE_POPUP: CommandText = CommandText::new(
        "Close [esc]",
        "close overlay (e.g commit, help)",
        CMD_GROUP_GENERAL,
    );
    ///
    pub static CLOSE_MSG: CommandText = CommandText::new(
        "Close [enter]",
        "close msg popup (e.g msg)",
        CMD_GROUP_GENERAL,
    )
    .hide_help();
    ///
    pub static SELECT_STAGING: CommandText = CommandText::new(
        "To stage [s]",
        "focus/select staging area",
        CMD_GROUP_GENERAL,
    );
    ///
    pub static SELECT_STATUS: CommandText = CommandText::new(
        "To files [1,2]",
        "focus/select file tree of staged or unstaged files",
        CMD_GROUP_GENERAL,
    );
    ///
    pub static SELECT_UNSTAGED: CommandText = CommandText::new(
        "To unstaged [w]",
        "focus/select unstaged area",
        CMD_GROUP_GENERAL,
    );
    ///
    pub static COMMIT_OPEN: CommandText = CommandText::new(
        "Commit [c]",
        "open commit popup (available in non-empty stage)",
        CMD_GROUP_COMMIT,
    );
    ///
    pub static COMMIT_OPEN_EDITOR: CommandText = CommandText::new(
        "Open editor [^e]",
        "open commit editor (available in non-empty stage)",
        CMD_GROUP_COMMIT,
    );
    ///
    pub static COMMIT_ENTER: CommandText = CommandText::new(
        "Commit [enter]",
        "commit (available when commit message is non-empty)",
        CMD_GROUP_COMMIT,
    );
    ///
    pub static COMMIT_AMEND: CommandText = CommandText::new(
        "Amend [^a]",
        "amend last commit",
        CMD_GROUP_COMMIT,
    );
    ///
    pub static EDIT_ITEM: CommandText = CommandText::new(
        "Edit Item [e]",
        "edit the currently selected file in an external editor",
        CMD_GROUP_CHANGES,
    );
    ///
    pub static STAGE_ITEM: CommandText = CommandText::new(
        "Stage Item [enter]",
        "stage currently selected file or entire path",
        CMD_GROUP_CHANGES,
    );
    ///
    pub static STAGE_ALL: CommandText = CommandText::new(
        "Stage All [a]",
        "stage all changes (in unstaged files)",
        CMD_GROUP_CHANGES,
    );
    ///
    pub static UNSTAGE_ITEM: CommandText = CommandText::new(
        "Unstage Item [enter]",
        "unstage currently selected file or entire path",
        CMD_GROUP_CHANGES,
    );
    ///
    pub static UNSTAGE_ALL: CommandText = CommandText::new(
        "Unstage all [a]",
        "unstage all files (in staged files)",
        CMD_GROUP_CHANGES,
    );
    ///
    pub static RESET_ITEM: CommandText = CommandText::new(
        "Reset Item [D]",
        "revert changes in selected file or entire path",
        CMD_GROUP_CHANGES,
    );
    ///
    pub static IGNORE_ITEM: CommandText = CommandText::new(
        "Ignore [i]",
        "Add file or path to .gitignore",
        CMD_GROUP_CHANGES,
    );
    ///
    pub static DIFF_FOCUS_LEFT: CommandText = CommandText::new(
        "Back [\u{2190}]", //←
        "view and select changed files",
        CMD_GROUP_GENERAL,
    );
    ///
    pub static DIFF_FOCUS_RIGHT: CommandText = CommandText::new(
        "Diff [\u{2192}]", //→
        "inspect file diff",
        CMD_GROUP_GENERAL,
    );
    ///
    pub static QUIT: CommandText = CommandText::new(
        "Quit [^c]",
        "quit gitui application",
        CMD_GROUP_GENERAL,
    );
    ///
    pub static RESET_CONFIRM: CommandText = CommandText::new(
        "Confirm [enter]",
        "resets the file in question",
        CMD_GROUP_GENERAL,
    );

    ///
    pub static STASHING_SAVE: CommandText = CommandText::new(
        "Save [s]",
        "opens stash name input popup",
        CMD_GROUP_STASHING,
    );
    ///
    pub static STASHING_TOGGLE_INDEXED: CommandText =
        CommandText::new(
            "Toggle Staged [i]",
            "toggle including staged files into stash",
            CMD_GROUP_STASHING,
        );
    ///
    pub static STASHING_TOGGLE_UNTRACKED: CommandText =
        CommandText::new(
            "Toggle Untracked [u]",
            "toggle including untracked files into stash",
            CMD_GROUP_STASHING,
        );
    ///
    pub static STASHING_CONFIRM_MSG: CommandText = CommandText::new(
        "Stash [enter]",
        "save files to stash",
        CMD_GROUP_STASHING,
    );
    ///
    pub static STASHLIST_APPLY: CommandText = CommandText::new(
        "Apply [enter]",
        "apply selected stash",
        CMD_GROUP_STASHES,
    );
    ///
    pub static STASHLIST_DROP: CommandText = CommandText::new(
        "Drop [D]",
        "drop selected stash",
        CMD_GROUP_STASHES,
    );
    ///
    pub static STASHLIST_INSPECT: CommandText = CommandText::new(
        "Inspect [\u{2192}]", //→
        "open stash commit details (allows to diff files)",
        CMD_GROUP_STASHES,
    );

    ///
    pub static LOG_DETAILS_TOGGLE: CommandText = CommandText::new(
        "Details [enter]",
        "open details of selected commit",
        CMD_GROUP_LOG,
    );
    ///
    pub static LOG_DETAILS_OPEN: CommandText = CommandText::new(
        "Inspect [\u{2192}]", //→
        "inspect selected commit in detail",
        CMD_GROUP_LOG,
    );
    ///
    pub static LOG_TAG_COMMIT: CommandText =
        CommandText::new("Tag [t]", "tag commit", CMD_GROUP_LOG);
    ///
    pub static TAG_COMMIT_CONFIRM_MSG: CommandText =
        CommandText::new("Tag [enter]", "tag commit", CMD_GROUP_LOG);
}
