//! sync git api for fetching a status

use crate::{
	error::Error,
	error::Result,
	sync::{config::untracked_files_config_repo, repository::repo},
};
use git2::{
	Delta, DiffDelta, Status, StatusEntry, StatusOptions, StatusShow,
};
use scopetime::scope_time;
use std::path::Path;

use super::{RepoPath, ShowUntrackedFilesConfig};

///
#[derive(Copy, Clone, Hash, PartialEq, Debug)]
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
	///
	Conflicted,
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
		} else if s.is_conflicted() {
			Self::Conflicted
		} else {
			Self::Modified
		}
	}
}

impl From<Delta> for StatusItemType {
	fn from(d: Delta) -> Self {
		match d {
			Delta::Added => Self::New,
			Delta::Deleted => Self::Deleted,
			Delta::Renamed => Self::Renamed,
			Delta::Typechange => Self::Typechange,
			_ => Self::Modified,
		}
	}
}

///
#[derive(Clone, Hash, PartialEq, Debug)]
pub struct StatusItem {
	///
	pub old_path: Option<String>,
	///
	pub new_path: String,
	///
	pub status: StatusItemType,
}

///
#[derive(Copy, Clone, Hash, PartialEq, Debug)]
pub enum StatusType {
	///
	WorkingDir,
	///
	Stage,
	///
	Both,
}

impl Default for StatusType {
	fn default() -> Self {
		Self::WorkingDir
	}
}

impl From<StatusType> for StatusShow {
	fn from(s: StatusType) -> Self {
		match s {
			StatusType::WorkingDir => Self::Workdir,
			StatusType::Stage => Self::Index,
			StatusType::Both => Self::IndexAndWorkdir,
		}
	}
}

fn get_diff<'a>(
	status_type: StatusType,
	status_entry: &'a StatusEntry,
) -> Option<DiffDelta<'a>> {
	(status_type != StatusType::WorkingDir)
		.then(|| status_entry.head_to_index())
		.or_else(|| {
			(status_type != StatusType::Stage)
				.then(|| status_entry.index_to_workdir())
		})
		.flatten()
}
///
pub fn is_workdir_clean(
	repo_path: &RepoPath,
	show_untracked: Option<ShowUntrackedFilesConfig>,
) -> Result<bool> {
	let repo = repo(repo_path)?;

	if repo.is_bare() && !repo.is_worktree() {
		return Ok(true);
	}

	let show_untracked = if let Some(config) = show_untracked {
		config
	} else {
		untracked_files_config_repo(&repo)?
	};

	let mut options = StatusOptions::default();
	options
		.show(StatusShow::Workdir)
		.update_index(true)
		.include_untracked(show_untracked.include_untracked())
		.renames_head_to_index(true)
		.recurse_untracked_dirs(
			show_untracked.recurse_untracked_dirs(),
		);

	let statuses = repo.statuses(Some(&mut options))?;

	Ok(statuses.is_empty())
}

/// gurantees sorting
pub fn get_status(
	repo_path: &RepoPath,
	status_type: StatusType,
	show_untracked: Option<ShowUntrackedFilesConfig>,
) -> Result<Vec<StatusItem>> {
	scope_time!("get_status");

	let repo = repo(repo_path)?;

	if repo.is_bare() && !repo.is_worktree() {
		return Ok(Vec::new());
	}

	let show_untracked = if let Some(config) = show_untracked {
		config
	} else {
		untracked_files_config_repo(&repo)?
	};

	let mut options = StatusOptions::default();
	options
		.show(status_type.into())
		.update_index(true)
		.include_untracked(show_untracked.include_untracked())
		.renames_head_to_index(status_type != StatusType::WorkingDir)
		.renames_index_to_workdir(status_type != StatusType::Stage)
		.recurse_untracked_dirs(
			show_untracked.recurse_untracked_dirs(),
		);

	let statuses = repo.statuses(Some(&mut options))?;

	let mut res = Vec::with_capacity(statuses.len());

	for e in statuses.iter() {
		let status: Status = e.status();

		let new_path = match get_diff(status_type, &e) {
			Some(diff) => diff
				.new_file()
				.path()
				.and_then(Path::to_str)
				.map(String::from)
				.ok_or_else(|| {
					Error::Generic(
						"failed to get path to diff's new file."
							.to_string(),
					)
				})?,
			None => e.path().map(String::from).ok_or_else(|| {
				Error::Generic(
					"failed to get the path to indexed file."
						.to_string(),
				)
			})?,
		};
		let old_path = get_diff(status_type, &e)
			.and_then(|diff| diff.old_file().path())
			.map(|path| {
				path.to_str().map(String::from).ok_or_else(|| {
					Error::Generic(
						"failed to get path to diff's new file."
							.to_string(),
					)
				})
			})
			.transpose()?;

		res.push(StatusItem {
			old_path,
			new_path,
			status: StatusItemType::from(status),
		});
	}

	res.sort_by(|a, b| {
		Path::new(a.new_path.as_str())
			.cmp(Path::new(b.new_path.as_str()))
	});

	Ok(res)
}

#[cfg(test)]
mod tests {
	use crate::{
		error::Result,
		sync::{
			commit, stage_add_file, stage_addremoved,
			status::{get_status, StatusItemType, StatusType},
			tests::repo_init_empty,
			RepoPath,
		},
	};
	use std::{
		fs::{self, File},
		io::Write,
		path::Path,
	};

	#[test]
	fn test_rename_file() -> Result<()> {
		let bar = Path::new("bar");
		let foo = Path::new("foo");
		let (_td, repo) = repo_init_empty()?;
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		let mut file = File::create(&root.join(bar))?;
		file.write_all(b"\x00")?;

		let statuses =
			get_status(repo_path, StatusType::Stage, None).unwrap();
		for diff in statuses.iter() {
			assert_eq!(diff.status, StatusItemType::New);
			assert_eq!(diff.old_path, Some("bar".to_string()));
			assert_eq!(diff.new_path, "bar".to_string());
		}

		stage_add_file(repo_path, bar)?;
		let _id = commit(repo_path, "")?;

		fs::rename(&root.join(bar), &root.join(foo))?;
		stage_add_file(repo_path, foo)?;
		stage_addremoved(repo_path, bar)?;
		let statuses =
			get_status(repo_path, StatusType::Stage, None).unwrap();
		for diff in statuses.iter() {
			assert_eq!(diff.status, StatusItemType::Renamed);
			assert_eq!(diff.old_path, Some("bar".to_string()));
			assert_eq!(diff.new_path, "foo".to_string());
		}

		fs::remove_file(&root.join(foo))?;
		stage_addremoved(repo_path, foo)?;
		let statuses =
			get_status(repo_path, StatusType::Stage, None).unwrap();
		for diff in statuses.iter() {
			assert_eq!(diff.status, StatusItemType::Deleted);
			assert_eq!(diff.old_path, Some("bar".to_string()));
			assert_eq!(diff.new_path, "bar".to_string());
		}
		Ok(())
	}
}
