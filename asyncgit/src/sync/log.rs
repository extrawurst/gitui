//TODO: WIP
#![allow(dead_code)]

use super::utils::repo;
use git2::{Commit, Error};
use scopetime::scope_time;

///
pub struct LogEntry {
    message: String,
    time: i64,
    author: String,
}

///
pub fn get_log(repo_path: &str) -> Result<Vec<LogEntry>, Error> {
    scope_time!("get_log");

    let repo = repo(repo_path);

    let mut walk = repo.revwalk()?;
    walk.push_head()?;

    let revwalk = walk.filter_map(|id| {
        if let Ok(id) = id {
            let commit = repo.find_commit(id);

            if let Ok(commit) = commit {
                return Some(commit);
            }
        }

        None
    });

    let res = revwalk
        .map(|c: Commit| LogEntry {
            message: String::from(c.message().unwrap()),
            author: String::from(c.author().name().unwrap()),
            time: c.time().seconds(),
        })
        .collect::<Vec<_>>();

    Ok(res)
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

        let res = get_log(repo_path).unwrap();

        assert_eq!(res.len(), 2);
        assert_eq!(res[0].message.as_str(), "commit2");
        assert_eq!(res[1].message.as_str(), "commit1");

        Ok(())
    }
}
