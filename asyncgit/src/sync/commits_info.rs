use super::utils::repo;
use crate::error::Result;
use git2::{Commit, Error, Oid};
use scopetime::scope_time;

/// identifies a single commit
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct CommitId(Oid);

impl CommitId {
    /// create new CommitId
    pub fn new(id: Oid) -> Self {
        Self(id)
    }

    ///
    pub(crate) fn get_oid(self) -> Oid {
        self.0
    }
}

impl ToString for CommitId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl Into<Oid> for CommitId {
    fn into(self) -> Oid {
        self.0
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
    repo_path: &str,
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
            let author = if let Some(name) = c.author().name() {
                String::from(name)
            } else {
                String::from("<unknown>")
            };
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
pub fn get_message(
    c: &Commit,
    message_length_limit: Option<usize>,
) -> String {
    let msg = String::from_utf8_lossy(c.message_bytes());
    let msg = msg.trim_start();

    if let Some(limit) = message_length_limit {
        limit_str(msg, limit).to_string()
    } else {
        msg.to_string()
    }
}

fn limit_str(s: &str, limit: usize) -> &str {
    if let Some(first) = s.lines().next() {
        &first[0..limit.min(first.len())]
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {

    use super::get_commits_info;
    use crate::error::Result;
    use crate::sync::{
        commit, stage_add_file, tests::repo_init_empty,
        utils::get_head_repo,
    };
    use std::{fs::File, io::Write, path::Path};

    #[test]
    fn test_log() -> Result<()> {
        let file_path = Path::new("foo");
        let (_td, repo) = repo_init_empty().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))?.write_all(b"a")?;
        stage_add_file(repo_path, file_path).unwrap();
        let c1 = commit(repo_path, "commit1").unwrap();
        File::create(&root.join(file_path))?.write_all(b"a")?;
        stage_add_file(repo_path, file_path).unwrap();
        let c2 = commit(repo_path, "commit2").unwrap();

        let res =
            get_commits_info(repo_path, &vec![c2, c1], 50).unwrap();

        assert_eq!(res.len(), 2);
        assert_eq!(res[0].message.as_str(), "commit2");
        assert_eq!(res[0].author.as_str(), "name");
        assert_eq!(res[1].message.as_str(), "commit1");

        Ok(())
    }

    #[test]
    fn test_invalid_utf8() -> Result<()> {
        let file_path = Path::new("foo");
        let (_td, repo) = repo_init_empty().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))?.write_all(b"a")?;
        stage_add_file(repo_path, file_path).unwrap();

        let msg = invalidstring::invalid_utf8("test msg");
        commit(repo_path, msg.as_str()).unwrap();

        let res = get_commits_info(
            repo_path,
            &vec![get_head_repo(&repo).unwrap().into()],
            50,
        )
        .unwrap();

        assert_eq!(res.len(), 1);
        dbg!(&res[0].message);
        assert_eq!(res[0].message.starts_with("test msg"), true);

        Ok(())
    }
}
