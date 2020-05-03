//TODO: WIP
#![allow(dead_code)]

use super::utils::repo;
use git2::{Commit, Error, Oid, Repository, Revwalk};
use log::debug;
use scopetime::scope_time;

///
pub struct LogEntry {
    pub message: String,
    pub time: i64,
    pub author: String,
}

///
pub fn get_log(
    repo_path: &str,
    limit: usize,
) -> Result<Vec<LogEntry>, Error> {
    scope_time!("get_log");

    let repo = repo(repo_path);

    let mut walk = repo.revwalk()?;
    // start at head
    walk.push_head()?;

    let revwalk = walk
        .filter_map(|id| {
            if let Ok(id) = id {
                let commit = repo.find_commit(id);

                if let Ok(commit) = commit {
                    return Some(commit);
                }
            }

            None
        })
        .take(limit);

    let res = revwalk
        .map(|c: Commit| {
            let message = get_message(&c);
            let author = if let Some(name) = c.author().name() {
                String::from(name)
            } else {
                String::from("<unknown>")
            };
            LogEntry {
                message,
                author,
                time: c.time().seconds(),
            }
        })
        .collect::<Vec<_>>();

    Ok(res)
}

///
pub struct Revlog<'a> {
    repo: &'a Repository,
    revwalk: Option<Revwalk<'a>>,
}

impl<'a> Revlog<'a> {
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

fn get_message(c: &Commit) -> String {
    if let Some(msg) = c.message() {
        limit_str(msg, 50)
    } else {
        String::from("<unknown>")
    }
}

fn limit_str(s: &str, limit: usize) -> String {
    if let Some(first) = s.lines().next() {
        first.chars().take(limit).collect::<String>()
    } else {
        String::new()
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
    fn test_log() -> Result<(), Error> {
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

        let res = get_log(repo_path, 100).unwrap();

        assert_eq!(res.len(), 2);
        assert_eq!(res[0].message.as_str(), "commit2");
        assert_eq!(res[0].author.as_str(), "name");
        assert_eq!(res[1].message.as_str(), "commit1");

        Ok(())
    }

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
        let mut walk = Revlog::new(&repo);
        walk.read(&mut items, 1);

        assert_eq!(items.len(), 1);

        Ok(())
    }

    #[test]
    fn test_revlog() -> Result<(), Error> {
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
        let mut walk = Revlog::new(&repo);
        walk.read(&mut items, 100);

        assert_eq!(items.len(), 2);

        let mut items = Vec::new();
        walk.read(&mut items, 100);

        assert_eq!(items.len(), 0);

        Ok(())
    }
}
