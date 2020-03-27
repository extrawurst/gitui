use super::utils::repo_at;
use git2::ObjectType;
use scopetime::scope_time;
use std::{path::Path, process::Command};

///
pub fn stage_reset(path: &Path) -> bool {
    stage_reset_at("./", path)
}

///
pub fn stage_reset_at(repo_path: &str, path: &Path) -> bool {
    scope_time!("stage_reset_at");

    let repo = repo_at(repo_path);

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
pub fn index_reset(path: &Path) -> bool {
    index_reset_at("./", path)
}

///
pub fn index_reset_at(repo_path: &str, path: &Path) -> bool {
    let cmd = format!("git checkout {}", path.to_str().unwrap());

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", cmd.as_str()])
            .current_dir(repo_path)
            .output()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(cmd.as_str())
            .current_dir(repo_path)
            .output()
    };

    if let Ok(out) = output {
        // dbg!(String::from_utf8(out.stderr.clone()).unwrap());
        String::from_utf8(out.stderr).unwrap()
            == "Updated 1 path from the index\n"
    } else {
        false
    }

    //------------------------------------
    //TODO: why is this broken with libgit2 ???
    //------------------------------------

    // scope_time!("index_reset");

    // let repo = repo_at(repo_path);

    // let mut checkout_opts = CheckoutBuilder::new();
    // checkout_opts
    //     .remove_untracked(true)
    //     .force()
    //     .update_index(false)
    //     .allow_conflicts(true)
    //     .path(&path);

    // if repo.checkout_head(Some(&mut checkout_opts)).is_ok() {
    //     return true;
    // }

    // false
}

#[cfg(test)]
mod tests {
    use super::index_reset_at;
    use crate::sync::{
        status::{get_index_at, StatusType},
        utils::stage_add_at,
    };
    use git2::Repository;
    use std::{fs::File, io::Write, path::Path};
    use tempfile::TempDir;

    pub fn repo_init() -> (TempDir, Repository) {
        let td = TempDir::new().unwrap();
        let repo = Repository::init(td.path()).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "name").unwrap();
            config.set_str("user.email", "email").unwrap();

            let mut index = repo.index().unwrap();
            let id = index.write_tree().unwrap();

            let tree = repo.find_tree(id).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                "initial",
                &tree,
                &[],
            )
            .unwrap();
        }
        (td, repo)
    }

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

        let res = get_index_at(repo_path, StatusType::WorkingDir);
        assert_eq!(res.len(), 0);

        let file_path = root.join("bar.txt");

        {
            File::create(&file_path)
                .unwrap()
                .write_all(HUNK_A.as_bytes())
                .unwrap();
        }

        stage_add_at(repo_path, Path::new("bar.txt"));

        // overwrite with next content
        {
            File::create(&file_path)
                .unwrap()
                .write_all(HUNK_B.as_bytes())
                .unwrap();
        }

        assert_eq!(
            get_index_at(repo_path, StatusType::Stage).len(),
            1
        );
        assert_eq!(
            get_index_at(repo_path, StatusType::WorkingDir).len(),
            1
        );

        let res = index_reset_at(repo_path, Path::new("bar.txt"));
        assert_eq!(res, true);

        assert_eq!(
            get_index_at(repo_path, StatusType::Stage).len(),
            1
        );
        assert_eq!(
            get_index_at(repo_path, StatusType::WorkingDir).len(),
            0
        );
    }
}
