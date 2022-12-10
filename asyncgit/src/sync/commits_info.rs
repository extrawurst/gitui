use super::RepoPath;
use crate::{error::Result, sync::repository::repo};
use git2::{Commit, Error, Oid};
use scopetime::scope_time;
use unicode_truncate::UnicodeTruncateStr;

/// identifies a single commit
#[derive(
	Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd,
)]
pub struct CommitId(Oid);

impl Default for CommitId {
	fn default() -> Self {
		Self(Oid::zero())
	}
}

impl CommitId {
	/// create new `CommitId`
	pub const fn new(id: Oid) -> Self {
		Self(id)
	}

	///
	pub(crate) const fn get_oid(self) -> Oid {
		self.0
	}

	/// 7 chars short hash
	pub fn get_short_string(&self) -> String {
		self.to_string().chars().take(7).collect()
	}
}

impl ToString for CommitId {
	fn to_string(&self) -> String {
		self.0.to_string()
	}
}

impl From<CommitId> for Oid {
	fn from(id: CommitId) -> Self {
		id.0
	}
}

impl From<Oid> for CommitId {
	fn from(id: Oid) -> Self {
		Self::new(id)
	}
}

///
#[derive(Debug)]
pub struct CommitInfo {
	///
	pub message: String,
	///
	pub time: i64,
	///
	pub author: String,
	///
	pub id: CommitId,
}

///
pub fn get_commits_info(
	repo_path: &RepoPath,
	ids: &[CommitId],
	message_length_limit: usize,
) -> Result<Vec<CommitInfo>> {
	scope_time!("get_commits_info");

	let repo = repo(repo_path)?;

	let commits = ids
		.iter()
		.map(|id| repo.find_commit((*id).into()))
		.collect::<std::result::Result<Vec<Commit>, Error>>()?
		.into_iter();

	let res = commits
		.map(|c: Commit| {
			let message = get_message(&c, Some(message_length_limit));
			let author = c.author().name().map_or_else(
				|| String::from("<unknown>"),
				String::from,
			);
			CommitInfo {
				message,
				author,
				time: c.time().seconds(),
				id: CommitId(c.id()),
			}
		})
		.collect::<Vec<_>>();

	Ok(res)
}

///
pub fn get_commit_info(
	repo_path: &RepoPath,
	commit_id: &CommitId,
) -> Result<CommitInfo> {
	scope_time!("get_commit_info");

	let repo = repo(repo_path)?;

	let commit = repo.find_commit((*commit_id).into())?;
	let author = commit.author();

	Ok(CommitInfo {
		message: commit.message().unwrap_or("").into(),
		author: author.name().unwrap_or("<unknown>").into(),
		time: commit.time().seconds(),
		id: CommitId(commit.id()),
	})
}

/// if `message_limit` is set the message will be
/// limited to the first line and truncated to fit
pub fn get_message(
	c: &Commit,
	message_limit: Option<usize>,
) -> String {
	let msg = String::from_utf8_lossy(c.message_bytes());
	let msg = msg.trim();

	message_limit.map_or_else(
		|| msg.to_string(),
		|limit| {
			let msg = msg.lines().next().unwrap_or_default();
			msg.unicode_truncate(limit).0.to_string()
		},
	)
}

#[cfg(test)]
mod tests {
	use super::get_commits_info;
	use crate::{
		error::Result,
		sync::{
			commit, stage_add_file, tests::repo_init_empty,
			utils::get_head_repo, RepoPath,
		},
	};
	use std::{fs::File, io::Write, path::Path};

	#[test]
	fn test_log() -> Result<()> {
		let file_path = Path::new("foo");
		let (_td, repo) = repo_init_empty().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		File::create(root.join(file_path))?.write_all(b"a")?;
		stage_add_file(repo_path, file_path).unwrap();
		let c1 = commit(repo_path, "commit1").unwrap();
		File::create(root.join(file_path))?.write_all(b"a")?;
		stage_add_file(repo_path, file_path).unwrap();
		let c2 = commit(repo_path, "commit2").unwrap();

		let res = get_commits_info(repo_path, &[c2, c1], 50).unwrap();

		assert_eq!(res.len(), 2);
		assert_eq!(res[0].message.as_str(), "commit2");
		assert_eq!(res[0].author.as_str(), "name");
		assert_eq!(res[1].message.as_str(), "commit1");

		Ok(())
	}

	#[test]
	fn test_log_first_msg_line() -> Result<()> {
		let file_path = Path::new("foo");
		let (_td, repo) = repo_init_empty().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		File::create(root.join(file_path))?.write_all(b"a")?;
		stage_add_file(repo_path, file_path).unwrap();
		let c1 = commit(repo_path, "subject\nbody").unwrap();

		let res = get_commits_info(repo_path, &[c1], 50).unwrap();

		assert_eq!(res.len(), 1);
		assert_eq!(res[0].message.as_str(), "subject");

		Ok(())
	}

	#[test]
	fn test_invalid_utf8() -> Result<()> {
		let file_path = Path::new("foo");
		let (_td, repo) = repo_init_empty().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		File::create(root.join(file_path))?.write_all(b"a")?;
		stage_add_file(repo_path, file_path).unwrap();

		let msg = invalidstring::invalid_utf8("test msg");
		commit(repo_path, msg.as_str()).unwrap();

		let res = get_commits_info(
			repo_path,
			&[get_head_repo(&repo).unwrap()],
			50,
		)
		.unwrap();

		assert_eq!(res.len(), 1);
		dbg!(&res[0].message);
		assert_eq!(res[0].message.starts_with("test msg"), true);

		Ok(())
	}
}
