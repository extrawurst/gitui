use super::utils::{repo, work_dir};
use crate::error::{Error, Result};
use git2::{build::CheckoutBuilder, ObjectType, Status};
use scopetime::scope_time;
use std::{fs, path::Path};

///
pub fn reset_stage(repo_path: &str, path: &str) -> Result<()> {
    scope_time!("reset_stage");

    let repo = repo(repo_path)?;

    let head = repo.head();

    if let Ok(reference) = head {
        let obj = repo.find_object(
            reference.target().ok_or_else(|| {
                Error::Generic(
                    "can't get reference to symbolic reference,"
                        .to_string(),
                )
            })?,
            Some(ObjectType::Commit),
        )?;

        repo.reset_default(Some(&obj), &[path])?;
    } else {
        repo.reset_default(None, &[path])?;
    }

    Ok(())
}

///
pub fn reset_workdir_file(repo_path: &str, path: &str) -> Result<()> {
    scope_time!("reset_workdir_file");

    let repo = repo(repo_path)?;

    let workdir = work_dir(&repo);

    // Note: early out for removing untracked files, due to bug in checkout_head code:
    // see https://github.com/libgit2/libgit2/issues/5089
    let status = repo.status_file(Path::new(path))?;

    if status == Status::WT_NEW
        || (status == Status::WT_MODIFIED | Status::INDEX_NEW)
    {
        fs::remove_file(Path::new(workdir).join(path))?;
    };

    if status == Status::WT_NEW {
        return Ok(());
    }

    let mut checkout_opts = CheckoutBuilder::new();
    checkout_opts
        .update_index(true) // windows: needs this to be true WTF?!
        .allow_conflicts(true)
        .force()
        .path(path);

    repo.checkout_index(None, Some(&mut checkout_opts))?;
    Ok(())
}

///
pub fn reset_workdir_folder(
    repo_path: &str,
    path: &str,
) -> Result<()> {
    scope_time!("reset_workdir_folder");

    let repo = repo(repo_path)?;

    let mut checkout_opts = CheckoutBuilder::new();
    checkout_opts
        .update_index(true) // windows: needs this to be true WTF?!
        .allow_conflicts(true)
        .remove_untracked(true)
        .force()
        .path(path);

    repo.checkout_index(None, Some(&mut checkout_opts))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        reset_stage, reset_workdir_file, reset_workdir_folder,
    };
    use crate::error::Result;
    use crate::sync::{
        status::{get_status, StatusType},
        tests::{
            debug_cmd_print, get_statuses, repo_init, repo_init_empty,
        },
        utils::{commit, stage_add_all, stage_add_file},
    };
    use std::{
        fs::{self, File},
        io::Write,
        path::Path,
    };

    static HUNK_A: &str = r"
1   start
2
3
4
5
6   middle
7
8
9
0
1   end";

    static HUNK_B: &str = r"
1   start
2   newa
3
4
5
6   middle
7
8
9
0   newb
1   end";

    #[test]
    fn test_reset_only_unstaged() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        let res =
            get_status(repo_path, StatusType::WorkingDir).unwrap();
        assert_eq!(res.len(), 0);

        let file_path = root.join("bar.txt");

        {
            File::create(&file_path)
                .unwrap()
                .write_all(HUNK_A.as_bytes())
                .unwrap();
        }

        debug_cmd_print(repo_path, "git status");

        stage_add_file(repo_path, Path::new("bar.txt")).unwrap();

        debug_cmd_print(repo_path, "git status");

        // overwrite with next content
        {
            File::create(&file_path)
                .unwrap()
                .write_all(HUNK_B.as_bytes())
                .unwrap();
        }

        debug_cmd_print(repo_path, "git status");

        assert_eq!(get_statuses(repo_path), (1, 1));

        reset_workdir_file(repo_path, "bar.txt").unwrap();

        debug_cmd_print(repo_path, "git status");

        assert_eq!(get_statuses(repo_path), (0, 1));
    }

    #[test]
    fn test_reset_untracked_in_subdir() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        {
            fs::create_dir(&root.join("foo")).unwrap();
            File::create(&root.join("foo/bar.txt"))
                .unwrap()
                .write_all(b"test\nfoo")
                .unwrap();
        }

        debug_cmd_print(repo_path, "git status");

        assert_eq!(get_statuses(repo_path), (1, 0));

        reset_workdir_file(repo_path, "foo/bar.txt").unwrap();

        debug_cmd_print(repo_path, "git status");

        assert_eq!(get_statuses(repo_path), (0, 0));
    }

    #[test]
    fn test_reset_folder() -> Result<()> {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        {
            fs::create_dir(&root.join("foo"))?;
            File::create(&root.join("foo/file1.txt"))?
                .write_all(b"file1")?;
            File::create(&root.join("foo/file2.txt"))?
                .write_all(b"file1")?;
            File::create(&root.join("file3.txt"))?
                .write_all(b"file3")?;
        }

        stage_add_all(repo_path, "*").unwrap();
        commit(repo_path, "msg").unwrap();

        {
            File::create(&root.join("foo/file1.txt"))?
                .write_all(b"file1\nadded line")?;
            fs::remove_file(&root.join("foo/file2.txt"))?;
            File::create(&root.join("foo/file4.txt"))?
                .write_all(b"file4")?;
            File::create(&root.join("foo/file5.txt"))?
                .write_all(b"file5")?;
            File::create(&root.join("file3.txt"))?
                .write_all(b"file3\nadded line")?;
        }

        assert_eq!(get_statuses(repo_path), (5, 0));

        stage_add_file(repo_path, Path::new("foo/file5.txt"))
            .unwrap();

        assert_eq!(get_statuses(repo_path), (4, 1));

        reset_workdir_folder(repo_path, "foo").unwrap();

        assert_eq!(get_statuses(repo_path), (1, 1));

        Ok(())
    }

    #[test]
    fn test_reset_untracked_in_subdir_and_index() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();
        let file = "foo/bar.txt";

        {
            fs::create_dir(&root.join("foo")).unwrap();
            File::create(&root.join(file))
                .unwrap()
                .write_all(b"test\nfoo")
                .unwrap();
        }

        debug_cmd_print(repo_path, "git status");

        debug_cmd_print(repo_path, "git add .");

        debug_cmd_print(repo_path, "git status");

        {
            File::create(&root.join(file))
                .unwrap()
                .write_all(b"test\nfoo\nnewend")
                .unwrap();
        }

        debug_cmd_print(repo_path, "git status");

        assert_eq!(get_statuses(repo_path), (1, 1));

        reset_workdir_file(repo_path, file).unwrap();

        debug_cmd_print(repo_path, "git status");

        assert_eq!(get_statuses(repo_path), (0, 1));
    }

    #[test]
    fn unstage_in_empty_repo() {
        let file_path = Path::new("foo.txt");
        let (_td, repo) = repo_init_empty().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))
            .unwrap()
            .write_all(b"test\nfoo")
            .unwrap();

        assert_eq!(get_statuses(repo_path), (1, 0));

        stage_add_file(repo_path, file_path).unwrap();

        assert_eq!(get_statuses(repo_path), (0, 1));

        reset_stage(repo_path, file_path.to_str().unwrap()).unwrap();

        assert_eq!(get_statuses(repo_path), (1, 0));
    }

    #[test]
    fn test_reset_untracked_in_subdir_with_cwd_in_subdir() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        {
            fs::create_dir(&root.join("foo")).unwrap();
            File::create(&root.join("foo/bar.txt"))
                .unwrap()
                .write_all(b"test\nfoo")
                .unwrap();
        }

        debug_cmd_print(repo_path, "git status");

        assert_eq!(get_statuses(repo_path), (1, 0));

        reset_workdir_file(
            &root.join("foo").as_os_str().to_str().unwrap(),
            "foo/bar.txt",
        )
        .unwrap();

        debug_cmd_print(repo_path, "git status");

        assert_eq!(get_statuses(repo_path), (0, 0));
    }
}
