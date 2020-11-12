//! sync git api (various methods)

use super::CommitId;
use crate::error::{Error, Result};
use git2::{IndexAddOption, Repository, RepositoryOpenFlags};
use scopetime::scope_time;
use std::path::Path;

///
#[derive(PartialEq, Debug, Clone)]
pub struct Head {
    ///
    pub name: String,
    ///
    pub id: CommitId,
}

///
pub fn is_repo(repo_path: &str) -> bool {
    Repository::open_ext(
        repo_path,
        RepositoryOpenFlags::empty(),
        Vec::<&Path>::new(),
    )
    .is_ok()
}

/// checks if the git repo at path `repo_path` is a bare repo
pub fn is_bare_repo(repo_path: &str) -> Result<bool> {
    let repo = Repository::open_ext(
        repo_path,
        RepositoryOpenFlags::empty(),
        Vec::<&Path>::new(),
    )?;

    Ok(repo.is_bare())
}

///
pub(crate) fn repo(repo_path: &str) -> Result<Repository> {
    let repo = Repository::open_ext(
        repo_path,
        RepositoryOpenFlags::empty(),
        Vec::<&Path>::new(),
    )?;

    if repo.is_bare() {
        return Err(Error::Generic("bare repo".to_string()));
    }

    Ok(repo)
}

///
pub(crate) fn work_dir(repo: &Repository) -> Result<&Path> {
    repo.workdir().map_or(Err(Error::NoWorkDir), |dir| Ok(dir))
}

///
pub fn repo_work_dir(repo_path: &str) -> Result<String> {
    let repo = repo(repo_path)?;
    if let Some(workdir) = work_dir(&repo)?.to_str() {
        Ok(workdir.to_string())
    } else {
        Err(Error::Generic("invalid workdir".to_string()))
    }
}

///
pub fn get_head(repo_path: &str) -> Result<CommitId> {
    let repo = repo(repo_path)?;
    get_head_repo(&repo)
}

///
pub fn get_head_tuple(repo_path: &str) -> Result<Head> {
    let repo = repo(repo_path)?;
    let id = get_head_repo(&repo)?;
    let name = get_head_refname(&repo)?;

    Ok(Head { name, id })
}

///
pub fn get_head_refname(repo: &Repository) -> Result<String> {
    let head = repo.head()?;
    let ref_name = bytes2string(head.name_bytes())?;

    Ok(ref_name)
}

///
pub fn get_head_repo(repo: &Repository) -> Result<CommitId> {
    scope_time!("get_head_repo");

    let head = repo.head()?.target();

    if let Some(head_id) = head {
        Ok(head_id.into())
    } else {
        Err(Error::NoHead)
    }
}

/// add a file diff from workingdir to stage (will not add removed files see `stage_addremoved`)
pub fn stage_add_file(repo_path: &str, path: &Path) -> Result<()> {
    scope_time!("stage_add_file");

    let repo = repo(repo_path)?;

    let mut index = repo.index()?;

    index.add_path(path)?;
    index.write()?;

    Ok(())
}

/// like `stage_add_file` but uses a pattern to match/glob multiple files/folders
pub fn stage_add_all(repo_path: &str, pattern: &str) -> Result<()> {
    scope_time!("stage_add_all");

    let repo = repo(repo_path)?;

    let mut index = repo.index()?;

    index.add_all(vec![pattern], IndexAddOption::DEFAULT, None)?;
    index.write()?;

    Ok(())
}

/// stage a removed file
pub fn stage_addremoved(repo_path: &str, path: &Path) -> Result<()> {
    scope_time!("stage_addremoved");

    let repo = repo(repo_path)?;

    let mut index = repo.index()?;

    index.remove_path(path)?;
    index.write()?;

    Ok(())
}

/// get string from config
pub fn get_config_string(
    repo_path: &str,
    key: &str,
) -> Result<Option<String>> {
    let repo = repo(repo_path)?;
    let cfg = repo.config()?;

    // this code doesnt match what the doc says regarding what
    // gets returned when but it actually works
    let entry_res = cfg.get_entry(key);

    let entry = match entry_res {
        Ok(ent) => ent,
        Err(_) => return Ok(None),
    };

    if !entry.has_value() {
        Ok(None)
    } else {
        Ok(entry.value().map(|s| s.to_string()))
    }
}
/// helper function
pub(crate) fn bytes2string(bytes: &[u8]) -> Result<String> {
    Ok(String::from_utf8(bytes.to_vec())?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::{
        commit,
        status::{get_status, StatusType},
        tests::{
            debug_cmd_print, get_statuses, repo_init, repo_init_empty,
        },
    };
    use std::{
        fs::{self, remove_file, File},
        io::Write,
        path::Path,
    };

    #[test]
    fn test_stage_add_smoke() {
        let file_path = Path::new("foo");
        let (_td, repo) = repo_init_empty().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        assert_eq!(
            stage_add_file(repo_path, file_path).is_ok(),
            false
        );
    }
    #[test]
    fn test_get_config() {
        let bad_dir_cfg =
            get_config_string("oodly_noodly", "this.doesnt.exist");
        assert!(bad_dir_cfg.is_err());

        let (_td, repo) = repo_init().unwrap();
        let path = repo.path();
        let rpath = path.as_os_str().to_str().unwrap();
        let bad_cfg = get_config_string(rpath, "this.doesnt.exist");
        assert!(bad_cfg.is_ok());
        assert!(bad_cfg.unwrap().is_none());
        // repo init sets user.name
        let good_cfg = get_config_string(rpath, "user.name");
        assert!(good_cfg.is_ok());
        assert!(good_cfg.unwrap().is_some());

    }
    #[test]
    fn test_staging_one_file() {
        let file_path = Path::new("file1.txt");
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))
            .unwrap()
            .write_all(b"test file1 content")
            .unwrap();

        File::create(&root.join(Path::new("file2.txt")))
            .unwrap()
            .write_all(b"test file2 content")
            .unwrap();

        assert_eq!(get_statuses(repo_path), (2, 0));

        stage_add_file(repo_path, file_path).unwrap();

        assert_eq!(get_statuses(repo_path), (1, 1));
    }

    #[test]
    fn test_staging_folder() -> Result<()> {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        let status_count = |s: StatusType| -> usize {
            get_status(repo_path, s, true).unwrap().len()
        };

        fs::create_dir_all(&root.join("a/d"))?;
        File::create(&root.join(Path::new("a/d/f1.txt")))?
            .write_all(b"foo")?;
        File::create(&root.join(Path::new("a/d/f2.txt")))?
            .write_all(b"foo")?;
        File::create(&root.join(Path::new("a/f3.txt")))?
            .write_all(b"foo")?;

        assert_eq!(status_count(StatusType::WorkingDir), 3);

        stage_add_all(repo_path, "a/d").unwrap();

        assert_eq!(status_count(StatusType::WorkingDir), 1);
        assert_eq!(status_count(StatusType::Stage), 2);

        Ok(())
    }

    #[test]
    fn test_staging_deleted_file() {
        let file_path = Path::new("file1.txt");
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        let status_count = |s: StatusType| -> usize {
            get_status(repo_path, s, true).unwrap().len()
        };

        let full_path = &root.join(file_path);

        File::create(full_path)
            .unwrap()
            .write_all(b"test file1 content")
            .unwrap();

        stage_add_file(repo_path, file_path).unwrap();

        commit(repo_path, "commit msg").unwrap();

        // delete the file now
        assert_eq!(remove_file(full_path).is_ok(), true);

        // deleted file in diff now
        assert_eq!(status_count(StatusType::WorkingDir), 1);

        stage_addremoved(repo_path, file_path).unwrap();

        assert_eq!(status_count(StatusType::WorkingDir), 0);
        assert_eq!(status_count(StatusType::Stage), 1);
    }

    // see https://github.com/extrawurst/gitui/issues/108
    #[test]
    fn test_staging_sub_git_folder() -> Result<()> {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        let status_count = |s: StatusType| -> usize {
            get_status(repo_path, s, true).unwrap().len()
        };

        let sub = &root.join("sub");

        fs::create_dir_all(sub)?;

        debug_cmd_print(sub.to_str().unwrap(), "git init subgit");

        File::create(sub.join("subgit/foo.txt"))
            .unwrap()
            .write_all(b"content")
            .unwrap();

        assert_eq!(status_count(StatusType::WorkingDir), 1);

        //expect to fail
        assert!(stage_add_all(repo_path, "sub").is_err());

        Ok(())
    }

    #[test]
    fn test_head_empty() -> Result<()> {
        let (_td, repo) = repo_init_empty()?;
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        assert_eq!(get_head(repo_path).is_ok(), false);

        Ok(())
    }

    #[test]
    fn test_head() -> Result<()> {
        let (_td, repo) = repo_init()?;
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        assert_eq!(get_head(repo_path).is_ok(), true);

        Ok(())
    }
}
