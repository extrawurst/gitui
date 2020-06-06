use super::{utils::repo, CommitId};
use crate::{error::Result, StatusItem, StatusItemType};
use git2::DiffDelta;
use scopetime::scope_time;

/// get all files that are part of a commit
pub fn get_commit_files(
    repo_path: &str,
    id: CommitId,
) -> Result<Vec<StatusItem>> {
    scope_time!("get_commit_files");

    let repo = repo(repo_path)?;

    let commit = repo.find_commit(id.into())?;
    let commit_tree = commit.tree()?;
    let parent = if commit.parent_count() > 0 {
        Some(repo.find_commit(commit.parent_id(0)?)?.tree()?)
    } else {
        None
    };

    let diff = repo.diff_tree_to_tree(
        parent.as_ref(),
        Some(&commit_tree),
        None,
    )?;

    let mut res = Vec::new();

    diff.foreach(
        &mut |delta: DiffDelta<'_>, _progress| {
            res.push(StatusItem {
                path: delta
                    .new_file()
                    .path()
                    .map(|p| p.to_str().unwrap_or("").to_string())
                    .unwrap_or_default(),
                status: StatusItemType::from(delta.status()),
            });
            true
        },
        None,
        None,
        None,
    )?;

    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::get_commit_files;
    use crate::{
        sync::{commit, stage_add_file, tests::repo_init, CommitId},
        StatusItemType,
    };
    use std::{fs::File, io::Write, path::Path};

    #[test]
    fn test_smoke() {
        let file_path = Path::new("file1.txt");
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))
            .unwrap()
            .write_all(b"test file1 content")
            .unwrap();

        stage_add_file(repo_path, file_path).unwrap();

        let id = commit(repo_path, "commit msg").unwrap();

        let diff =
            get_commit_files(repo_path, CommitId::new(id)).unwrap();

        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].status, StatusItemType::New);
    }
}
