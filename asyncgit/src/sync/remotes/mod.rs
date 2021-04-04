//!

pub(crate) mod push;
pub(crate) mod tags;

use self::push::ProgressNotification;
use super::cred::BasicAuthCredential;
use crate::{
    error::{Error, Result},
    sync::utils,
};
use crossbeam_channel::Sender;
use git2::{Direction, FetchOptions, Repository};
use push::remote_callbacks;
use scopetime::scope_time;

/// origin
pub const DEFAULT_REMOTE_NAME: &str = "origin";

///
pub fn get_remotes(repo_path: &str) -> Result<Vec<String>> {
    scope_time!("get_remotes");

    let repo = utils::repo(repo_path)?;
    let remotes = repo.remotes()?;
    let remotes: Vec<String> =
        remotes.iter().flatten().map(String::from).collect();

    Ok(remotes)
}

///
pub fn get_remote_branches(
    repo_path: &str,
    remote: &str,
    basic_credential: Option<BasicAuthCredential>,
) -> Result<Vec<String>> {
    scope_time!("get_remote_branches");

    let repo = utils::repo(repo_path)?;

    let mut remote = repo.find_remote(remote)?;

    remote.connect_auth(
        Direction::Fetch,
        Some(remote_callbacks(None, basic_credential)),
        None,
    )?;

    let list = remote.list()?;

    let res = list
        .iter()
        .filter_map(|entry| {
            let name = entry.name();
            if name.starts_with("refs/heads/") {
                Some(String::from(name))
            } else {
                None
            }
        })
        .collect();

    Ok(res)
}

/// tries to find origin or the only remote that is defined if any
/// in case of multiple remotes and none named *origin* we fail
pub fn get_default_remote(repo_path: &str) -> Result<String> {
    let repo = utils::repo(repo_path)?;
    get_default_remote_in_repo(&repo)
}

/// see `get_default_remote`
pub(crate) fn get_default_remote_in_repo(
    repo: &Repository,
) -> Result<String> {
    scope_time!("get_default_remote_in_repo");

    let remotes = repo.remotes()?;

    // if `origin` exists return that
    let found_origin = remotes.iter().any(|r| {
        r.map(|r| r == DEFAULT_REMOTE_NAME).unwrap_or_default()
    });
    if found_origin {
        return Ok(DEFAULT_REMOTE_NAME.into());
    }

    //if only one remote exists pick that
    if remotes.len() == 1 {
        let first_remote = remotes
            .iter()
            .next()
            .flatten()
            .map(String::from)
            .ok_or_else(|| {
                Error::Generic("no remote found".into())
            })?;

        return Ok(first_remote);
    }

    //inconclusive
    Err(Error::NoDefaultRemoteFound)
}

///
pub(crate) fn fetch_origin(
    repo_path: &str,
    branch: &str,
    basic_credential: Option<BasicAuthCredential>,
    progress_sender: Option<Sender<ProgressNotification>>,
) -> Result<usize> {
    scope_time!("fetch_origin");

    let repo = utils::repo(repo_path)?;
    let mut remote =
        repo.find_remote(&get_default_remote_in_repo(&repo)?)?;

    let mut options = FetchOptions::new();
    options.remote_callbacks(remote_callbacks(
        progress_sender,
        basic_credential,
    ));

    remote.fetch(&[branch], Some(&mut options), None)?;

    Ok(remote.stats().received_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::{
        branch::get_branches_info,
        create_branch,
        remotes::push::push,
        tests::{
            debug_cmd_print, repo_clone, repo_init_bare,
            write_commit_file,
        },
    };
    use tempfile::TempDir;

    #[test]
    fn test_smoke() {
        let td = TempDir::new().unwrap();

        debug_cmd_print(
            td.path().as_os_str().to_str().unwrap(),
            "git clone https://github.com/extrawurst/brewdump.git",
        );

        let repo_path = td.path().join("brewdump");
        let repo_path = repo_path.as_os_str().to_str().unwrap();

        let remotes = get_remotes(repo_path).unwrap();

        assert_eq!(remotes, vec![String::from("origin")]);

        fetch_origin(repo_path, "master", None, None).unwrap();
    }

    #[test]
    fn test_default_remote() {
        let td = TempDir::new().unwrap();

        debug_cmd_print(
            td.path().as_os_str().to_str().unwrap(),
            "git clone https://github.com/extrawurst/brewdump.git",
        );

        debug_cmd_print(
            td.path().as_os_str().to_str().unwrap(),
            "cd brewdump && git remote add second https://github.com/extrawurst/brewdump.git",
        );

        let repo_path = td.path().join("brewdump");
        let repo_path = repo_path.as_os_str().to_str().unwrap();

        let remotes = get_remotes(repo_path).unwrap();

        assert_eq!(
            remotes,
            vec![String::from("origin"), String::from("second")]
        );

        let first = get_default_remote_in_repo(
            &utils::repo(repo_path).unwrap(),
        )
        .unwrap();
        assert_eq!(first, String::from("origin"));
    }

    #[test]
    fn test_default_remote_out_of_order() {
        let td = TempDir::new().unwrap();

        debug_cmd_print(
            td.path().as_os_str().to_str().unwrap(),
            "git clone https://github.com/extrawurst/brewdump.git",
        );

        debug_cmd_print(
            td.path().as_os_str().to_str().unwrap(),
            "cd brewdump && git remote rename origin alternate",
        );

        debug_cmd_print(
            td.path().as_os_str().to_str().unwrap(),
            "cd brewdump && git remote add origin https://github.com/extrawurst/brewdump.git",
        );

        let repo_path = td.path().join("brewdump");
        let repo_path = repo_path.as_os_str().to_str().unwrap();

        //NOTE: aparently remotes are not chronolically sorted but alphabetically
        let remotes = get_remotes(repo_path).unwrap();

        assert_eq!(
            remotes,
            vec![String::from("alternate"), String::from("origin")]
        );

        let first = get_default_remote_in_repo(
            &utils::repo(repo_path).unwrap(),
        )
        .unwrap();
        assert_eq!(first, String::from("origin"));
    }

    #[test]
    fn test_default_remote_inconclusive() {
        let td = TempDir::new().unwrap();

        debug_cmd_print(
            td.path().as_os_str().to_str().unwrap(),
            "git clone https://github.com/extrawurst/brewdump.git",
        );

        debug_cmd_print(
            td.path().as_os_str().to_str().unwrap(),
            "cd brewdump && git remote rename origin alternate",
        );

        debug_cmd_print(
            td.path().as_os_str().to_str().unwrap(),
            "cd brewdump && git remote add someremote https://github.com/extrawurst/brewdump.git",
        );

        let repo_path = td.path().join("brewdump");
        let repo_path = repo_path.as_os_str().to_str().unwrap();

        let remotes = get_remotes(repo_path).unwrap();
        assert_eq!(
            remotes,
            vec![
                String::from("alternate"),
                String::from("someremote")
            ]
        );

        let res = get_default_remote_in_repo(
            &utils::repo(repo_path).unwrap(),
        );
        assert_eq!(res.is_err(), true);
        assert!(matches!(res, Err(Error::NoDefaultRemoteFound)));
    }

    #[test]
    fn test_remote_branches() {
        let (r1_dir, _repo) = repo_init_bare().unwrap();

        let (clone1_dir, clone1) =
            repo_clone(r1_dir.path().to_str().unwrap()).unwrap();

        let clone1_dir = clone1_dir.path().to_str().unwrap();

        // clone1

        write_commit_file(&clone1, "test.txt", "test", "commit1");

        push(clone1_dir, "origin", "master", false, None, None)
            .unwrap();

        create_branch(clone1_dir, "foo").unwrap();

        write_commit_file(&clone1, "test.txt", "test2", "commit2");

        push(clone1_dir, "origin", "foo", false, None, None).unwrap();

        // clone2

        let (clone2_dir, _clone2) =
            repo_clone(r1_dir.path().to_str().unwrap()).unwrap();

        let clone2_dir = clone2_dir.path().to_str().unwrap();

        let local_branches = get_branches_info(clone2_dir).unwrap();

        assert_eq!(local_branches.len(), 1);

        let branches =
            get_remote_branches(clone2_dir, "origin", None).unwrap();

        assert_eq!(
            &branches,
            &["refs/heads/foo", "refs/heads/master",]
        );
    }
}
