#![allow(dead_code)]
use super::{CommitId, SharedCommitFilterFn};
use crate::error::Result;
use git2::{Commit, Oid, Repository};
use std::{
	cell::RefCell,
	cmp::Ordering,
	collections::{BinaryHeap, HashSet},
};

struct TimeOrderedCommit<'a>(Commit<'a>);

impl<'a> Eq for TimeOrderedCommit<'a> {}

impl<'a> PartialEq for TimeOrderedCommit<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.0.time().eq(&other.0.time())
	}
}

impl<'a> PartialOrd for TimeOrderedCommit<'a> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl<'a> Ord for TimeOrderedCommit<'a> {
	fn cmp(&self, other: &Self) -> Ordering {
		self.0.time().cmp(&other.0.time())
	}
}

///
pub struct LogWalker<'a> {
	commits: BinaryHeap<TimeOrderedCommit<'a>>,
	visited: HashSet<Oid>,
	limit: Option<usize>,
	repo: &'a Repository,
	filter: Option<SharedCommitFilterFn>,
}

impl<'a> LogWalker<'a> {
	///
	pub fn new(
		repo: &'a Repository,
		limit: Option<usize>,
	) -> Result<Self> {
		let c = repo.head()?.peel_to_commit()?;

		let mut commits = BinaryHeap::with_capacity(10);
		commits.push(TimeOrderedCommit(c));

		Ok(Self {
			commits,
			limit,
			visited: HashSet::with_capacity(1000),
			repo,
			filter: None,
		})
	}

	///
	pub fn visited(&self) -> usize {
		self.visited.len()
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
	pub fn read(
		&mut self,
		out: Option<&RefCell<Vec<CommitId>>>,
	) -> Result<usize> {
		let mut count = 0_usize;

		while let Some(c) = self.commits.pop() {
			for p in c.0.parents() {
				self.visit(p);
			}

			let id: CommitId = c.0.id().into();
			let commit_should_be_included =
				if let Some(ref filter) = self.filter {
					filter(self.repo, &id)?
				} else {
					true
				};

			if commit_should_be_included {
				if let Some(out) = out {
					out.borrow_mut().push(id);
				}
			}

			count += 1;
			if self
				.limit
				.map(|limit| limit == count)
				.unwrap_or_default()
			{
				break;
			}
		}

		Ok(count)
	}

	//
	fn visit(&mut self, c: Commit<'a>) {
		if !self.visited.contains(&c.id()) {
			self.visited.insert(c.id());
			self.commits.push(TimeOrderedCommit(c));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::error::Result;
	use crate::sync::commit_files::{
		commit_contains_file, commit_detect_file_rename,
	};
	use crate::sync::commit_filter::{SearchFields, SearchOptions};
	use crate::sync::tests::{rename_file, write_commit_file};
	use crate::sync::{
		commit, get_commits_info, stage_add_file,
		tests::repo_init_empty,
	};
	use crate::sync::{
		filter_commit_by_search, stage_add_all, LogFilterSearch,
		LogFilterSearchOptions, RepoPath,
	};
	use pretty_assertions::assert_eq;
	use std::sync::{Arc, RwLock};
	use std::{fs::File, io::Write, path::Path};

	fn diff_contains_file(
		file_path: Arc<RwLock<String>>,
	) -> SharedCommitFilterFn {
		Arc::new(Box::new(
			move |repo: &Repository,
			      commit_id: &CommitId|
			      -> Result<bool> {
				let current_file_path = file_path.read()?.to_string();

				if let Some(delta) = commit_contains_file(
					repo,
					*commit_id,
					current_file_path.as_str(),
				)? {
					if matches!(delta, git2::Delta::Added) {
						let rename = commit_detect_file_rename(
							repo,
							*commit_id,
							current_file_path.as_str(),
						)?;

						if let Some(old_name) = rename {
							(*file_path.write()?) = old_name;
						}
					}

					return Ok(true);
				}

				Ok(false)
			},
		))
	}

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

		let items = RefCell::new(Vec::new());
		let mut walk = LogWalker::new(&repo, Some(1))?;
		walk.read(Some(&items)).unwrap();
		let items = items.take();

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

		let items = RefCell::new(Vec::new());
		let mut walk = LogWalker::new(&repo, Some(100))?;
		walk.read(Some(&items)).unwrap();
		let items = items.take();

		let info = get_commits_info(repo_path, &items, 50).unwrap();
		dbg!(&info);

		assert_eq!(items.len(), 2);
		assert_eq!(items[0], oid2);

		let items = RefCell::new(Vec::new());
		walk.read(Some(&items)).unwrap();
		let items = items.take();

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

		let file_path = Arc::new(RwLock::new(String::from("baz")));
		let diff_contains_baz = diff_contains_file(file_path);

		let items = RefCell::new(Vec::new());
		let mut walker = LogWalker::new(&repo, Some(100))?
			.filter(Some(diff_contains_baz));
		walker.read(Some(&items)).unwrap();
		let items = items.take();

		assert_eq!(items.len(), 1);
		assert_eq!(items[0], second_commit_id);

		let items = RefCell::new(Vec::new());
		walker.read(Some(&items)).unwrap();
		let items = items.take();

		assert_eq!(items.len(), 0);

		let file_path = Arc::new(RwLock::new(String::from("bar")));
		let diff_contains_bar = diff_contains_file(file_path);

		let items = RefCell::new(Vec::new());
		let mut walker = LogWalker::new(&repo, Some(100))?
			.filter(Some(diff_contains_bar));
		walker.read(Some(&items)).unwrap();
		let items = items.take();

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

		let items = RefCell::new(Vec::new());
		let mut walker = LogWalker::new(&repo, Some(100))
			.unwrap()
			.filter(Some(log_filter));
		walker.read(Some(&items)).unwrap();
		let items = items.take();

		assert_eq!(items.len(), 1);
		assert_eq!(items[0], second_commit_id);

		let log_filter = filter_commit_by_search(
			LogFilterSearch::new(LogFilterSearchOptions {
				fields: SearchFields::FILENAMES,
				options: SearchOptions::FUZZY_SEARCH,
				search_pattern: String::from("fo"),
			}),
		);

		let items = RefCell::new(Vec::new());
		let mut walker = LogWalker::new(&repo, Some(100))
			.unwrap()
			.filter(Some(log_filter));
		walker.read(Some(&items)).unwrap();

		assert_eq!(items.take().len(), 2);
	}

	#[test]
	fn test_logwalker_with_filter_rename() {
		let (td, repo) = repo_init_empty().unwrap();
		let repo_path: RepoPath = td.path().into();

		write_commit_file(&repo, "foo.txt", "foobar", "c1");
		rename_file(&repo, "foo.txt", "bar.txt");
		stage_add_all(
			&repo_path,
			"*",
			Some(crate::sync::ShowUntrackedFilesConfig::All),
		)
		.unwrap();
		let rename_commit = commit(&repo_path, "c2").unwrap();

		write_commit_file(&repo, "bar.txt", "new content", "c3");

		let file_path =
			Arc::new(RwLock::new(String::from("bar.txt")));
		let log_filter = diff_contains_file(file_path.clone());

		let items = RefCell::new(Vec::new());
		let mut walker = LogWalker::new(&repo, Some(100))
			.unwrap()
			.filter(Some(log_filter));
		walker.read(Some(&items)).unwrap();
		let items = items.take();

		assert_eq!(items.len(), 3);
		assert_eq!(items[1], rename_commit);

		assert_eq!(file_path.read().unwrap().as_str(), "foo.txt");
	}
}
