#![allow(dead_code)]
use super::{CommitId, SharedCommitFilterFn};
use crate::error::Result;
use gix::{
	revision::Walk, traverse::commit::simple::Sorting, Repository,
};

///
pub struct LogWalker<'a> {
	walk: Walk<'a>,
	limit: usize,
	visited: usize,
	filter: Option<SharedCommitFilterFn>,
}

impl<'a> LogWalker<'a> {
	///
	pub fn new(
		repo: &'a mut Repository,
		limit: usize,
	) -> Result<Self> {
		repo.object_cache_size_if_unset(2_usize.pow(14));

		let commit = repo
			.head()
			.expect("repo.head() failed")
			.peel_to_commit_in_place()
			.expect("peel_to_commit_in_place failed");

		let tips = [commit.id];

		let platform = repo
			.rev_walk(tips)
			.sorting(Sorting::ByCommitTimeNewestFirst)
			.use_commit_graph(false);

		let walk = platform
			.all()
			.expect("platform.all() failed");

		Ok(Self {
			walk,
			limit,
			visited: 0,
			filter: None,
		})
	}

	///
	pub fn visited(&self) -> usize {
		self.visited
	}

	///
	#[must_use]
	pub fn filter(
		self,
		filter: Option<SharedCommitFilterFn>,
	) -> Self {
		Self { filter, ..self }
	}

	///
	pub fn read(&mut self, out: &mut Vec<CommitId>) -> Result<usize> {
		let mut count = 0_usize;

		while let Some(Ok(info)) = self.walk.next() {
			let commit_id = Into::<CommitId>::into(info.id);

			out.push(commit_id);

			count += 1;

			if count == self.limit {
				break;
			}
		}

		self.visited += count;

		Ok(count)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::error::Result;
	use crate::sync::commit_filter::{SearchFields, SearchOptions};
	use crate::sync::tests::write_commit_file;
	use crate::sync::{
		commit, get_commits_info, stage_add_file,
		tests::repo_init_empty,
	};
	use crate::sync::{
		diff_contains_file, filter_commit_by_search, LogFilterSearch,
		LogFilterSearchOptions, RepoPath,
	};
	use pretty_assertions::assert_eq;
	use std::{fs::File, io::Write, path::Path};

	#[test]
	fn test_limit() -> Result<()> {
		let file_path = Path::new("foo");
		let (_td, repo) = repo_init_empty().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		File::create(root.join(file_path))?.write_all(b"a")?;
		stage_add_file(repo_path, file_path).unwrap();
		commit(repo_path, "commit1").unwrap();
		File::create(root.join(file_path))?.write_all(b"a")?;
		stage_add_file(repo_path, file_path).unwrap();
		let oid2 = commit(repo_path, "commit2").unwrap();

		let mut items = Vec::new();
		let mut repo = gix::open(&repo_path.gitpath())
			.expect("gix::open failed");
		let mut walk = LogWalker::new(&mut repo, 1)?;
		walk.read(&mut items).unwrap();

		assert_eq!(items.len(), 1);
		assert_eq!(items[0], oid2);

		Ok(())
	}

	#[test]
	fn test_logwalker() -> Result<()> {
		let file_path = Path::new("foo");
		let (_td, repo) = repo_init_empty().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		File::create(root.join(file_path))?.write_all(b"a")?;
		stage_add_file(repo_path, file_path).unwrap();
		commit(repo_path, "commit1").unwrap();
		File::create(root.join(file_path))?.write_all(b"a")?;
		stage_add_file(repo_path, file_path).unwrap();
		let oid2 = commit(repo_path, "commit2").unwrap();

		let mut items = Vec::new();
		let mut repo = gix::open(&repo_path.gitpath())
			.expect("gix::open failed");
		let mut walk = LogWalker::new(&mut repo, 100)?;
		walk.read(&mut items).unwrap();

		let info = get_commits_info(repo_path, &items, 50).unwrap();
		dbg!(&info);

		assert_eq!(items.len(), 2);
		assert_eq!(items[0], oid2);

		let mut items = Vec::new();
		walk.read(&mut items).unwrap();

		assert_eq!(items.len(), 0);

		Ok(())
	}

	#[test]
	fn test_logwalker_with_filter() -> Result<()> {
		let file_path = Path::new("foo");
		let second_file_path = Path::new("baz");
		let (_td, repo) = repo_init_empty().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: RepoPath =
			root.as_os_str().to_str().unwrap().into();

		File::create(root.join(file_path))?.write_all(b"a")?;
		stage_add_file(&repo_path, file_path).unwrap();

		let _first_commit_id = commit(&repo_path, "commit1").unwrap();

		File::create(root.join(second_file_path))?.write_all(b"a")?;
		stage_add_file(&repo_path, second_file_path).unwrap();

		let second_commit_id = commit(&repo_path, "commit2").unwrap();

		File::create(root.join(file_path))?.write_all(b"b")?;
		stage_add_file(&repo_path, file_path).unwrap();

		let _third_commit_id = commit(&repo_path, "commit3").unwrap();

		let diff_contains_baz = diff_contains_file("baz".into());

		let mut items = Vec::new();
		let mut repo = gix::open(&repo_path.gitpath())
			.expect("gix::open failed");
		let mut walker = LogWalker::new(&mut repo, 100)?
			.filter(Some(diff_contains_baz));
		walker.read(&mut items).unwrap();

		assert_eq!(items.len(), 1);
		assert_eq!(items[0], second_commit_id);

		let mut items = Vec::new();
		walker.read(&mut items).unwrap();

		assert_eq!(items.len(), 0);

		let diff_contains_bar = diff_contains_file("bar".into());

		let mut items = Vec::new();
		let mut repo = gix::open(&repo_path.gitpath())
			.expect("gix::open failed");
		let mut walker = LogWalker::new(&mut repo, 100)?
			.filter(Some(diff_contains_bar));
		walker.read(&mut items).unwrap();

		assert_eq!(items.len(), 0);

		Ok(())
	}

	#[test]
	fn test_logwalker_with_filter_search() {
		let (_td, repo) = repo_init_empty().unwrap();

		write_commit_file(&repo, "foo", "a", "commit1");
		let second_commit_id = write_commit_file(
			&repo,
			"baz",
			"a",
			"my commit msg (#2)",
		);
		write_commit_file(&repo, "foo", "b", "commit3");

		let log_filter = filter_commit_by_search(
			LogFilterSearch::new(LogFilterSearchOptions {
				fields: SearchFields::MESSAGE_SUMMARY,
				options: SearchOptions::FUZZY_SEARCH,
				search_pattern: String::from("my msg"),
			}),
		);

		let repo_path = repo.path();

		let mut items = Vec::new();
		let mut repo =
			gix::open(repo_path).expect("gix::open failed");
		let mut walker = LogWalker::new(&mut repo, 100)
			.unwrap()
			.filter(Some(log_filter));
		walker.read(&mut items).unwrap();

		assert_eq!(items.len(), 1);
		assert_eq!(items[0], second_commit_id);

		let log_filter = filter_commit_by_search(
			LogFilterSearch::new(LogFilterSearchOptions {
				fields: SearchFields::FILENAMES,
				options: SearchOptions::FUZZY_SEARCH,
				search_pattern: String::from("fo"),
			}),
		);

		let mut items = Vec::new();
		let mut repo =
			gix::open(repo_path).expect("gix::open failed");
		let mut walker = LogWalker::new(&mut repo, 100)
			.unwrap()
			.filter(Some(log_filter));
		walker.read(&mut items).unwrap();

		assert_eq!(items.len(), 2);
	}
}
