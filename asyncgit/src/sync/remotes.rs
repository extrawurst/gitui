//!

use crate::{error::Result, sync::utils};
use crossbeam_channel::Sender;
use git2::{
    Cred, FetchOptions, PackBuilderStage, PushOptions,
    RemoteCallbacks,
};
use scopetime::scope_time;

///
#[derive(Debug, Clone, Copy)]
pub enum ProgressNotification {
    ///
    PushTransfer {
        ///
        current: usize,
        ///
        total: usize,
        ///
        bytes: usize,
    },
    ///
    Packing {
        ///
        stage: PackBuilderStage,
        ///
        total: usize,
        ///
        current: usize,
    },
    ///
    Done,
}

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
pub fn fetch_origin(repo_path: &str, branch: &str) -> Result<usize> {
    scope_time!("fetch_origin");

    let repo = utils::repo(repo_path)?;
    let mut remote = repo.find_remote("origin")?;

    let mut options = FetchOptions::new();
    options.remote_callbacks(remote_callbacks(None));

    remote.fetch(&[branch], Some(&mut options), None)?;

    Ok(remote.stats().received_bytes())
}

///
pub fn push(
    repo_path: &str,
    remote: &str,
    branch: &str,
    progress_sender: Sender<ProgressNotification>,
) -> Result<()> {
    scope_time!("push_origin");

    let repo = utils::repo(repo_path)?;
    let mut remote = repo.find_remote(remote)?;

    let mut options = PushOptions::new();

    options.remote_callbacks(remote_callbacks(Some(progress_sender)));
    options.packbuilder_parallelism(0);

    remote.push(&[branch], Some(&mut options))?;

    Ok(())
}

fn remote_callbacks<'a>(
    sender: Option<Sender<ProgressNotification>>,
) -> RemoteCallbacks<'a> {
    let mut callbacks = RemoteCallbacks::new();
    let sender_clone = sender.clone();
    callbacks.push_transfer_progress(move |current, total, bytes| {
        sender_clone.clone().map(|sender| {
            sender.send(ProgressNotification::PushTransfer {
                current,
                total,
                bytes,
            })
        });

        log::debug!("progress: {}/{} ({} B)", current, total, bytes,);
    });
    callbacks.update_tips(|name, a, b| {
        log::debug!("update: '{}' [{}] [{}]", name, a, b);
        true
    });
    callbacks.transfer_progress(|p| {
        log::debug!(
            "transfer progress: {} B / {} / {} ",
            p.received_bytes(),
            p.received_objects(),
            p.total_objects()
        );
        true
    });
    callbacks.pack_progress(move |stage, current, total| {
        sender.clone().map(|sender| {
            sender.send(ProgressNotification::Packing {
                stage,
                total,
                current,
            })
        });

        log::debug!("packing: {:?} - {}/{}", stage, current, total);
    });
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
