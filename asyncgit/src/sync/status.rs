//! sync git api for fetching a status

use crate::sync::utils;
use git2::{Status, StatusOptions, StatusShow};
use scopetime::scope_time;

///
#[derive(Copy, Clone, Hash)]
pub enum StatusItemType {
    ///
    New,
    ///
    Modified,
    ///
    Deleted,
    ///
    Renamed,
    ///
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

///
#[derive(Default, Clone, Hash)]
pub struct StatusItem {
    ///
    pub path: String,
    ///
    pub status: Option<StatusItemType>,
}

///
#[derive(Copy, Clone)]
pub enum StatusType {
    ///
    WorkingDir,
    ///
    Stage,
}

impl Into<StatusShow> for StatusType {
    fn into(self) -> StatusShow {
        match self {
            StatusType::WorkingDir => StatusShow::Workdir,
            StatusType::Stage => StatusShow::Index,
        }
    }
}

///
pub fn get_index(status_type: StatusType) -> Vec<StatusItem> {
    scope_time!("get_index");

    let repo = utils::repo();

    let statuses = repo
        .statuses(Some(
            StatusOptions::default()
                .show(status_type.into())
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
