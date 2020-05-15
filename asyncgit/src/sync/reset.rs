use super::utils::repo;
use crate::error::{Error, Result};
use git2::{build::CheckoutBuilder, ObjectType, Status};
use scopetime::scope_time;
use std::{fs, path::Path};

///
pub fn reset_stage(repo_path: &str, path: &Path) -> Result<bool> {
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

    Ok(true)
}

///
pub fn reset_workdir_file(
    repo_path: &str,
    path: &str,
) -> Result<bool> {
    scope_time!("reset_workdir_file");

    let repo = repo(repo_path)?;

    // Note: early out for removing untracked files, due to bug in checkout_head code:
    // see https://github.com/libgit2/libgit2/issues/5089
    if let Ok(status) = repo.status_file(Path::new(path)) {
        let removed_file_wd = if status == Status::WT_NEW
            || (status == Status::WT_MODIFIED | Status::INDEX_NEW)
        {
            Ok(fs::remove_file(Path::new(repo_path).join(path))
                .is_ok())
        } else {
            Ok(false)
        };

        if status == Status::WT_NEW {
            return removed_file_wd;
        }

        let mut checkout_opts = CheckoutBuilder::new();
        checkout_opts
            .update_index(true) // windows: needs this to be true WTF?!
            .allow_conflicts(true)
            .force()
            .path(path);

        Ok(repo
            .checkout_index(None, Some(&mut checkout_opts))
            .is_ok())
    } else {
        Ok(false)
    }
}

///
pub fn reset_workdir_folder(
    repo_path: &str,
    path: &str,
) -> Result<bool> {
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
    Ok(true)
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

        debug_cmd_print(repo_path, "git status").unwrap();

        stage_add_file(repo_path, Path::new("bar.txt")).unwrap();

        debug_cmd_print(repo_path, "git status").unwrap();

        // overwrite with next content
        {
            File::create(&file_path)
                .unwrap()
                .write_all(HUNK_B.as_bytes())
                .unwrap();
        }

        debug_cmd_print(repo_path, "git status").unwrap();

        assert_eq!(get_statuses(repo_path).unwrap(), (1, 1));

        let res = reset_workdir_file(repo_path, "bar.txt").unwrap();
        assert_eq!(res, true);

        debug_cmd_print(repo_path, "git status").unwrap();

        assert_eq!(get_statuses(repo_path).unwrap(), (0, 1));
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

        debug_cmd_print(repo_path, "git status").unwrap();

        assert_eq!(get_statuses(repo_path).unwrap(), (1, 0));

        let res =
            reset_workdir_file(repo_path, "foo/bar.txt").unwrap();
        assert_eq!(res, true);

        debug_cmd_print(repo_path, "git status").unwrap();

        assert_eq!(get_statuses(repo_path).unwrap(), (0, 0));
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

        assert!(stage_add_all(repo_path, "*").unwrap());
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

        assert_eq!(get_statuses(repo_path).unwrap(), (5, 0));

        stage_add_file(repo_path, Path::new("foo/file5.txt"))
            .unwrap();

        assert_eq!(get_statuses(repo_path).unwrap(), (4, 1));

        assert!(reset_workdir_folder(repo_path, "foo").unwrap());

        assert_eq!(get_statuses(repo_path).unwrap(), (1, 1));

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

        debug_cmd_print(repo_path, "git status").unwrap();

        debug_cmd_print(repo_path, "git add .").unwrap();

        debug_cmd_print(repo_path, "git status").unwrap();

        {
            File::create(&root.join(file))
                .unwrap()
                .write_all(b"test\nfoo\nnewend")
                .unwrap();
        }

        debug_cmd_print(repo_path, "git status").unwrap();

        assert_eq!(get_statuses(repo_path).unwrap(), (1, 1));

        let res = reset_workdir_file(repo_path, file).unwrap();
        assert_eq!(res, true);

        debug_cmd_print(repo_path, "git status").unwrap();

        assert_eq!(get_statuses(repo_path).unwrap(), (0, 1));
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

        assert_eq!(get_statuses(repo_path).unwrap(), (1, 0));

        assert_eq!(
            stage_add_file(repo_path, file_path).unwrap(),
            true
        );

        assert_eq!(get_statuses(repo_path).unwrap(), (0, 1));

        assert_eq!(reset_stage(repo_path, file_path).unwrap(), true);

        assert_eq!(get_statuses(repo_path).unwrap(), (1, 0));
    }
}
