use super::{utils::repo, CommitId};
use crate::error::Result;
use scopetime::scope_time;

///
pub fn amend(
    repo_path: &str,
    id: CommitId,
    msg: &str,
) -> Result<CommitId> {
    scope_time!("commit");

    let repo = repo(repo_path)?;
    let commit = repo.find_commit(id.into())?;

    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let new_id = commit.amend(
        Some("HEAD"),
        None,
        None,
        None,
        Some(msg),
        Some(&tree),
    )?;

    Ok(CommitId::new(new_id))
}

#[cfg(test)]
mod tests {

    use crate::error::Result;
    use crate::sync::{
        commit, get_commit_details, get_commit_files, stage_add_file,
        tests::repo_init_empty, utils::get_head, CommitId, LogWalker,
    };
    use commit::amend;
    use git2::Repository;
    use std::{fs::File, io::Write, path::Path};

    fn count_commits(repo: &Repository, max: usize) -> usize {
        let mut items = Vec::new();
        let mut walk = LogWalker::new(&repo);
        walk.read(&mut items, max).unwrap();
        items.len()
    }

    #[test]
    fn test_amend() -> Result<()> {
        let file_path1 = Path::new("foo");
        let file_path2 = Path::new("foo2");
        let (_td, repo) = repo_init_empty()?;
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path1))?.write_all(b"test1")?;

        stage_add_file(repo_path, file_path1)?;
        let id = commit(repo_path, "commit msg")?;

        assert_eq!(count_commits(&repo, 10), 1);

        File::create(&root.join(file_path2))?.write_all(b"test2")?;

        stage_add_file(repo_path, file_path2)?;

        let new_id = amend(repo_path, CommitId::new(id), "amended")?;

        assert_eq!(count_commits(&repo, 10), 1);

        let details = get_commit_details(repo_path, new_id)?;
        assert_eq!(details.message.unwrap().subject, "amended");

        let files = get_commit_files(repo_path, new_id)?;

        assert_eq!(files.len(), 2);

        let head = get_head(repo_path)?;

        assert_eq!(head, new_id);

        Ok(())
    }
}
