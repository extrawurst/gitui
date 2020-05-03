use git2::{Oid, Repository, Revwalk};
use log::debug;

///
pub struct LogWalker<'a> {
    repo: &'a Repository,
    revwalk: Option<Revwalk<'a>>,
}

impl<'a> LogWalker<'a> {
    ///
    pub fn new(repo: &'a Repository) -> Self {
        Self {
            repo,
            revwalk: None,
        }
    }

    ///
    pub fn read(
        &mut self,
        out: &mut Vec<Oid>,
        limit: usize,
    ) -> usize {
        let mut count = 0_usize;

        if self.revwalk.is_none() {
            let walk = self.repo.revwalk().unwrap();
            self.revwalk = Some(walk);
            if let Some(ref mut walk) = self.revwalk {
                walk.push_head().unwrap();
            }
        }

        if let Some(walk) = &mut self.revwalk {
            for id in walk {
                if let Ok(id) = id {
                    // if repo.find_commit(id).is_ok()
                    {
                        out.push(id);
                        count += 1;

                        if count == limit {
                            break;
                        }
                    }
                }
            }
        }

        debug!("done walk: {}", count);

        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::{
        commit, stage_add_file, tests::repo_init_empty,
    };
    use std::{
        fs::File,
        io::{Error, Write},
        path::Path,
    };

    #[test]
    fn test_limit() -> Result<(), Error> {
        let file_path = Path::new("foo");
        let (_td, repo) = repo_init_empty();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))?.write_all(b"a")?;
        stage_add_file(repo_path, file_path);
        commit(repo_path, "commit1");
        File::create(&root.join(file_path))?.write_all(b"a")?;
        stage_add_file(repo_path, file_path);
        commit(repo_path, "commit2");

        let mut items = Vec::new();
        let mut walk = LogWalker::new(&repo);
        walk.read(&mut items, 1);

        assert_eq!(items.len(), 1);

        Ok(())
    }

    #[test]
    fn test_logwalker() -> Result<(), Error> {
        let file_path = Path::new("foo");
        let (_td, repo) = repo_init_empty();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))?.write_all(b"a")?;
        stage_add_file(repo_path, file_path);
        commit(repo_path, "commit1");
        File::create(&root.join(file_path))?.write_all(b"a")?;
        stage_add_file(repo_path, file_path);
        commit(repo_path, "commit2");

        let mut items = Vec::new();
        let mut walk = LogWalker::new(&repo);
        walk.read(&mut items, 100);

        assert_eq!(items.len(), 2);

        let mut items = Vec::new();
        walk.read(&mut items, 100);

        assert_eq!(items.len(), 0);

        Ok(())
    }
}
