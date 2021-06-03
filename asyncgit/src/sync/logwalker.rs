#![allow(clippy::missing_panics_doc)]

use super::CommitId;
use crate::error::Result;
use git2::{Repository, Revwalk};

///
pub struct LogWalker<'a> {
    repo: &'a Repository,
    revwalk: Option<Revwalk<'a>>,
    limit: usize,
}

impl<'a> LogWalker<'a> {
    ///
    pub const fn new(repo: &'a Repository, limit: usize) -> Self {
        Self {
            repo,
            revwalk: None,
            limit,
        }
    }

    ///
    pub fn read(&mut self, out: &mut Vec<CommitId>) -> Result<usize> {
        let mut count = 0_usize;

        if self.revwalk.is_none() {
            let mut walk = self.repo.revwalk()?;

            walk.push_head()?;

            self.revwalk = Some(walk);
        }

        if let Some(ref mut walk) = self.revwalk {
            for id in walk.into_iter().flatten() {
                out.push(id.into());
                count += 1;

                if count == self.limit {
                    break;
                }
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::{
        commit, get_commits_info, stage_add_file,
        tests::repo_init_empty,
    };
    use pretty_assertions::assert_eq;
    use std::{fs::File, io::Write, path::Path};

    #[test]
    fn test_limit() -> Result<()> {
        let file_path = Path::new("foo");
        let (_td, repo) = repo_init_empty().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))?.write_all(b"a")?;
        stage_add_file(repo_path, file_path).unwrap();
        commit(repo_path, "commit1").unwrap();
        File::create(&root.join(file_path))?.write_all(b"a")?;
        stage_add_file(repo_path, file_path).unwrap();
        let oid2 = commit(repo_path, "commit2").unwrap();

        let mut items = Vec::new();
        let mut walk = LogWalker::new(&repo, 1);
        walk.read(&mut items).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0], oid2.into());

        Ok(())
    }

    #[test]
    fn test_logwalker() -> Result<()> {
        let file_path = Path::new("foo");
        let (_td, repo) = repo_init_empty().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))?.write_all(b"a")?;
        stage_add_file(repo_path, file_path).unwrap();
        commit(repo_path, "commit1").unwrap();
        File::create(&root.join(file_path))?.write_all(b"a")?;
        stage_add_file(repo_path, file_path).unwrap();
        let oid2 = commit(repo_path, "commit2").unwrap();

        let mut items = Vec::new();
        let mut walk = LogWalker::new(&repo, 100);
        walk.read(&mut items).unwrap();

        let info = get_commits_info(repo_path, &items, 50).unwrap();
        dbg!(&info);

        assert_eq!(items.len(), 2);
        assert_eq!(items[0], oid2.into());

        let mut items = Vec::new();
        walk.read(&mut items).unwrap();

        assert_eq!(items.len(), 0);

        Ok(())
    }

    // fn walk_all_commits(repo: &Repository) -> Vec<CommitId> {
    //     let mut items = Vec::new();
    //     let mut walk = LogWalker::new(&repo).mode(Mode::AllRefs);
    //     walk.read(&mut items, 10).unwrap();
    //     items
    // }

    // #[test]
    // fn test_multiple_branches() {
    //     let (td, repo) = repo_init_empty().unwrap();
    //     let repo_path = td.path().to_string_lossy();

    //     let c1 = write_commit_file_at(
    //         &repo,
    //         "test.txt",
    //         "",
    //         "c1",
    //         Time::new(1, 0),
    //     );

    //     let items = walk_all_commits(&repo);

    //     assert_eq!(items, vec![c1]);

    //     let b1 = create_branch(&repo_path, "b1").unwrap();

    //     let c2 = write_commit_file_at(
    //         &repo,
    //         "test2.txt",
    //         "",
    //         "c2",
    //         Time::new(2, 0),
    //     );

    //     let items = walk_all_commits(&repo);
    //     assert_eq!(items, vec![c2, c1]);

    //     let _b2 = create_branch(&repo_path, "b2").unwrap();

    //     let c3 = write_commit_file_at(
    //         &repo,
    //         "test3.txt",
    //         "",
    //         "c3",
    //         Time::new(3, 0),
    //     );

    //     let items = walk_all_commits(&repo);
    //     assert_eq!(items, vec![c2, c3, c1]);

    //     checkout_branch(&repo_path, &b1).unwrap();

    //     let items = walk_all_commits(&repo);
    //     assert_eq!(items, vec![c2, c3, c1]);
    // }
}
