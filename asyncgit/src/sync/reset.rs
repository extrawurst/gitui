use super::utils::repo;
use git2::{build::CheckoutBuilder, ObjectType, Status};
use scopetime::scope_time;
use std::{fs, path::Path};

///
pub fn reset_stage(repo_path: &str, path: &Path) -> bool {
    scope_time!("reset_stage");

    let repo = repo(repo_path);

    let reference = repo.head().unwrap();
    let obj = repo
        .find_object(
            reference.target().unwrap(),
            Some(ObjectType::Commit),
        )
        .unwrap();

    if repo.reset_default(Some(&obj), &[path]).is_ok() {
        return true;
    }

    false
}

///
pub fn reset_workdir(repo_path: &str, path: &Path) -> bool {
    scope_time!("reset_workdir");

    let repo = repo(repo_path);

    // Note: early out for removing untracked files, due to bug in checkout_head code:
    // see https://github.com/libgit2/libgit2/issues/5089
    if let Ok(status) = repo.status_file(&path) {
        let removed_file_wd = if status == Status::WT_NEW
            || (status == Status::WT_MODIFIED | Status::INDEX_NEW)
        {
            fs::remove_file(Path::new(repo_path).join(path)).is_ok()
        } else {
            false
        };

        if status == Status::WT_NEW {
            return removed_file_wd;
        }

        if status != Status::WT_MODIFIED | Status::INDEX_NEW {
            let mut checkout_opts = CheckoutBuilder::new();
            checkout_opts
                .remove_untracked(true)
                .force()
                .update_index(false)
                .path(&path);

            //first reset working dir file
            repo.checkout_head(Some(&mut checkout_opts)).unwrap();
        }

        let mut checkout_opts = CheckoutBuilder::new();
        checkout_opts
            .update_index(true) // windows: needs this to be true WTF?!
            .allow_conflicts(true)
            .path(&path);

        // now reset staged changes back to working dir
        repo.checkout_index(None, Some(&mut checkout_opts)).unwrap();

        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::reset_workdir;
    use crate::sync::{
        status::{get_status, StatusType},
        tests::{debug_cmd_print, repo_init},
        utils::stage_add,
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
        let (_td, repo) = repo_init();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        let res = get_status(repo_path, StatusType::WorkingDir);
        assert_eq!(res.len(), 0);

        let file_path = root.join("bar.txt");

        {
            File::create(&file_path)
                .unwrap()
                .write_all(HUNK_A.as_bytes())
                .unwrap();
        }

        debug_cmd_print(repo_path, "git status");

        stage_add(repo_path, Path::new("bar.txt"));

        debug_cmd_print(repo_path, "git status");

        // overwrite with next content
        {
            File::create(&file_path)
                .unwrap()
                .write_all(HUNK_B.as_bytes())
                .unwrap();
        }

        debug_cmd_print(repo_path, "git status");

        assert_eq!(get_status(repo_path, StatusType::Stage).len(), 1);
        assert_eq!(
            get_status(repo_path, StatusType::WorkingDir).len(),
            1
        );

        let res = reset_workdir(repo_path, Path::new("bar.txt"));
        assert_eq!(res, true);

        debug_cmd_print(repo_path, "git status");

        assert_eq!(get_status(repo_path, StatusType::Stage).len(), 1);
        assert_eq!(
            get_status(repo_path, StatusType::WorkingDir).len(),
            0
        );
    }

    #[test]
    fn test_reset_untracked_in_subdir() {
        let (_td, repo) = repo_init();
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

        assert_eq!(
            get_status(repo_path, StatusType::WorkingDir).len(),
            1
        );

        let res = reset_workdir(repo_path, Path::new("foo/bar.txt"));
        assert_eq!(res, true);

        debug_cmd_print(repo_path, "git status");

        assert_eq!(
            get_status(repo_path, StatusType::WorkingDir).len(),
            0
        );
    }
}
