use crate::git_utils;
use git2::{Status, StatusOptions, StatusShow};
use scopetime::scope_time;

#[derive(PartialEq, Copy, Clone)]
pub enum StatusItemType {
    New,
    Modified,
    Deleted,
    Renamed,
    Typechange,
}

impl From<Status> for StatusItemType {
    fn from(s: Status) -> Self {
        if s.is_index_new() || s.is_wt_new() {
            Self::New
        } else if s.is_index_deleted() || s.is_wt_deleted() {
            Self::Deleted
        } else if s.is_index_renamed() || s.is_wt_renamed() {
            Self::Renamed
        } else if s.is_index_typechange() || s.is_wt_typechange() {
            Self::Typechange
        } else {
            Self::Modified
        }
    }
}

#[derive(Default, PartialEq, Clone)]
pub struct StatusItem {
    pub path: String,
    pub status: Option<StatusItemType>,
}

pub fn get_index(show: StatusShow) -> Vec<StatusItem> {
    scope_time!("get_index");

    let repo = git_utils::repo();

    let statuses = repo
        .statuses(Some(
            StatusOptions::default()
                .show(show)
                .include_untracked(true)
                .renames_head_to_index(true)
                .recurse_untracked_dirs(true),
        ))
        .unwrap();

    let mut res = Vec::with_capacity(statuses.len());

    for e in statuses.iter() {
        let status: Status = e.status();

        let path = if let Some(diff) = e.head_to_index() {
            String::from(
                diff.new_file().path().unwrap().to_str().unwrap(),
            )
        } else {
            e.path().unwrap().to_string()
        };

        res.push(StatusItem {
            path,
            status: Some(StatusItemType::from(status)),
        });
    }

    res
}
