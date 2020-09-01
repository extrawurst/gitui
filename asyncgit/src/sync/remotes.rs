//!

use crate::{error::Result, sync::utils};
use git2::{Cred, FetchOptions, PushOptions, RemoteCallbacks};
use scopetime::scope_time;

///
pub fn get_remotes(repo_path: &str) -> Result<Vec<String>> {
    scope_time!("get_remotes");

    let repo = utils::repo(repo_path)?;
    let remotes = repo.remotes()?;
    let remotes: Vec<String> =
        remotes.iter().filter_map(|s| s).map(String::from).collect();

    Ok(remotes)
}

///
pub fn remote_push_master(repo_path: &str) -> Result<()> {
    scope_time!("remote_push_master");

    let repo = utils::repo(repo_path)?;
    let mut remote = repo.find_remote("origin")?;

    remote.push(&["refs/heads/master"], None)?;

    Ok(())
}

///
pub fn fetch_origin(repo_path: &str, branch: &str) -> Result<usize> {
    scope_time!("remote_fetch_master");

    let repo = utils::repo(repo_path)?;
    let mut remote = repo.find_remote("origin")?;

    let mut options = FetchOptions::new();
    options.remote_callbacks(remote_callbacks());

    remote.fetch(&[branch], Some(&mut options), None)?;

    Ok(remote.stats().received_bytes())
}

///
pub fn push_origin(repo_path: &str, branch: &str) -> Result<usize> {
    scope_time!("push_origin");

    let repo = utils::repo(repo_path)?;
    let mut remote = repo.find_remote("origin")?;

    let mut options = PushOptions::new();
    options.remote_callbacks(remote_callbacks());

    remote.push(&[branch], Some(&mut options))?;

    Ok(remote.stats().received_bytes())
}

fn remote_callbacks<'a>() -> RemoteCallbacks<'a> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|url, username_from_url, allowed_types| {
        log::debug!(
            "creds: '{}' {:?} ({:?})",
            url,
            username_from_url,
            allowed_types
        );

        Cred::ssh_key_from_agent(
            username_from_url.expect("username not found"),
        )
    });

    callbacks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::tests::debug_cmd_print;
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

        fetch_origin(repo_path, "master").unwrap();
    }
}
