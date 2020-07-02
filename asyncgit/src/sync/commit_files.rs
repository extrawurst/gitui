use super::{stash::is_stash_commit, utils::repo, CommitId};
use crate::{error::Result, StatusItem, StatusItemType, CWD};
use git2::{Diff, DiffDelta, DiffOptions, Repository};
use scopetime::scope_time;

/// get all files that are part of a commit
pub fn get_commit_files(
    repo_path: &str,
    id: CommitId,
) -> Result<Vec<StatusItem>> {
    scope_time!("get_commit_files");

    let repo = repo(repo_path)?;

    let diff = get_commit_diff(&repo, id, None)?;

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

    // stash commits have parent commits containing untracked files and if we want to show
    // these files as if they were actually in the stash commit  we have to have some specific
    // handling regarding these special stash commits.
    // more info can be found at https://stackoverflow.com/questions/51275777/why-does-git-stash-pop-say-that-it-could-not-restore-untracked-files-from-stash/51276389#51276389
    if is_stash_commit(repo_path, &id)? {
        let commit = repo.find_commit(id.into())?;
        let untracked_commit = commit.parent_id(2)?;

        let mut untracked_files = get_commit_files(
            repo_path,
            CommitId::new(untracked_commit),
        )?;

        res.append(&mut untracked_files);
    }

    Ok(res)
}

///
pub(crate) fn get_commit_diff(
    repo: &Repository,
    id: CommitId,
    pathspec: Option<String>,
) -> Result<Diff<'_>> {
    // scope_time!("get_commit_diff");

    let commit = repo.find_commit(id.into())?;
    let commit_tree = commit.tree()?;
    let parent = if commit.parent_count() > 0 {
        Some(repo.find_commit(commit.parent_id(0)?)?.tree()?)
    } else {
        None
    };

    let mut opt = pathspec.as_ref().map(|p| {
        let mut opts = DiffOptions::new();
        opts.pathspec(p);
        opts.show_binary(true);
        opts
    });

    let diff = repo.diff_tree_to_tree(
        parent.as_ref(),
        Some(&commit_tree),
        opt.as_mut(),
    )?;

    if diff.deltas().len() == 0 && is_stash_commit(CWD, &id)? {
        let untracked_commit = commit.parent_id(2)?;

        return get_commit_diff(
            repo,
            CommitId::new(untracked_commit),
            pathspec,
        );
    }

    Ok(diff)
}

#[cfg(test)]
mod tests {
    use super::get_commit_files;
    use crate::{
        error::Result,
        sync::{
            commit, stage_add_file, stash_save, tests::repo_init,
        },
        StatusItemType,
    };
    use std::{fs::File, io::Write, path::Path};

    #[test]
    fn test_smoke() -> Result<()> {
        let file_path = Path::new("file1.txt");
        let (_td, repo) = repo_init()?;
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))?
            .write_all(b"test file1 content")?;

        stage_add_file(repo_path, file_path)?;

        let id = commit(repo_path, "commit msg")?;

        let diff = get_commit_files(repo_path, id)?;

        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].status, StatusItemType::New);

        Ok(())
    }

    #[test]
    fn test_stashed_untracked() -> Result<()> {
        let file_path = Path::new("file1.txt");
        let (_td, repo) = repo_init()?;
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))?
            .write_all(b"test file1 content")?;

        let id = stash_save(repo_path, None, true, false)?;

        //TODO: https://github.com/extrawurst/gitui/issues/130
        // `get_commit_diff` actually needs to merge the regular diff
        // and a third parent diff containing the untracked files
        let diff = get_commit_files(repo_path, id)?;

        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].status, StatusItemType::New);

        Ok(())
    }
}
