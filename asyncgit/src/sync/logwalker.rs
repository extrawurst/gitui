use crate::error::Result;
use git2::{Oid, Repository, Revwalk};

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
    ) -> Result<usize> {
        let mut count = 0_usize;

        if self.revwalk.is_none() {
            let mut walk = self.repo.revwalk()?;
            walk.push_head()?;
            self.revwalk = Some(walk);
        }

        if let Some(ref mut walk) = self.revwalk {
            for id in walk {
                if let Ok(id) = id {
                    out.push(id);
                    count += 1;

                    if count == limit {
                        break;
                    }
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
        let mut walk = LogWalker::new(&repo);
        walk.read(&mut items, 1).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0], oid2);

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
        let mut walk = LogWalker::new(&repo);
        walk.read(&mut items, 100).unwrap();

        let info = get_commits_info(repo_path, &items, 50).unwrap();
        dbg!(&info);

        assert_eq!(items.len(), 2);
        assert_eq!(items[0], oid2);

        let mut items = Vec::new();
        walk.read(&mut items, 100).unwrap();

        assert_eq!(items.len(), 0);

        Ok(())
    }
}
