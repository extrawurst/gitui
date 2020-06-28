use super::{get_head, utils::repo, CommitId};
use crate::error::Result;
use git2::{ErrorCode, Repository, Signature};
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

/// Wrap Repository::signature to allow unknown user.name.
///
/// See <https://github.com/extrawurst/gitui/issues/79>.
fn signature_allow_undefined_name(
    repo: &Repository,
) -> std::result::Result<Signature<'_>, git2::Error> {
    match repo.signature() {
        Err(e) if e.code() == ErrorCode::NotFound => {
            let config = repo.config()?;
            Signature::now(
                config.get_str("user.name").unwrap_or("unknown"),
                config.get_str("user.email")?,
            )
        }

        v => v,
    }
}

/// this does not run any git hooks
pub fn commit(repo_path: &str, msg: &str) -> Result<CommitId> {
    scope_time!("commit");

    let repo = repo(repo_path)?;

    let signature = signature_allow_undefined_name(&repo)?;
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let parents = if let Ok(id) = get_head(repo_path) {
        vec![repo.find_commit(id.into())?]
    } else {
        Vec::new()
    };

    let parents = parents.iter().collect::<Vec<_>>();

    Ok(repo
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            msg,
            &tree,
            parents.as_slice(),
        )?
        .into())
}

#[cfg(test)]
mod tests {

    use crate::error::Result;
    use crate::sync::{
        commit, get_commit_details, get_commit_files, stage_add_file,
        tests::{get_statuses, repo_init, repo_init_empty},
        utils::get_head,
        LogWalker,
    };
    use commit::amend;
    use git2::Repository;
    use std::{fs::File, io::Write, path::Path};
    use tempfile::TempDir;

    fn count_commits(repo: &Repository, max: usize) -> usize {
        let mut items = Vec::new();
        let mut walk = LogWalker::new(&repo);
        walk.read(&mut items, max).unwrap();
        items.len()
    }

    #[test]
    fn test_commit() {
        let file_path = Path::new("foo");
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))
            .unwrap()
            .write_all(b"test\nfoo")
            .unwrap();

        assert_eq!(get_statuses(repo_path), (1, 0));

        stage_add_file(repo_path, file_path).unwrap();

        assert_eq!(get_statuses(repo_path), (0, 1));

        commit(repo_path, "commit msg").unwrap();

        assert_eq!(get_statuses(repo_path), (0, 0));
    }

    #[test]
    fn test_commit_in_empty_repo() {
        let file_path = Path::new("foo");
        let (_td, repo) = repo_init_empty().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        assert_eq!(get_statuses(repo_path), (0, 0));

        File::create(&root.join(file_path))
            .unwrap()
            .write_all(b"test\nfoo")
            .unwrap();

        assert_eq!(get_statuses(repo_path), (1, 0));

        stage_add_file(repo_path, file_path).unwrap();

        assert_eq!(get_statuses(repo_path), (0, 1));

        commit(repo_path, "commit msg").unwrap();

        assert_eq!(get_statuses(repo_path), (0, 0));
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

        let new_id = amend(repo_path, id, "amended")?;

        assert_eq!(count_commits(&repo, 10), 1);

        let details = get_commit_details(repo_path, new_id)?;
        assert_eq!(details.message.unwrap().subject, "amended");

        let files = get_commit_files(repo_path, new_id)?;

        assert_eq!(files.len(), 2);

        let head = get_head(repo_path)?;

        assert_eq!(head, new_id);

        Ok(())
    }

    #[test]
    fn test_commit_unknown_signature() {
        let file_path = Path::new("foo");
        let td = TempDir::new().unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))
            .unwrap()
            .write_all(b"test\nfoo")
            .unwrap();

        stage_add_file(repo_path, file_path).unwrap();

        let id = commit(repo_path, "commit msg").unwrap();

        let details = get_commit_details(repo_path, id).unwrap();

        assert_eq!(details.author.name, "unknown");
    }
}
