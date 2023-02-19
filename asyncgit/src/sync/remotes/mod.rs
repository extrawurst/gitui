//!

mod callbacks;
pub(crate) mod push;
pub(crate) mod tags;

use crate::{
	error::{Error, Result},
	sync::{
		cred::BasicAuthCredential,
		remotes::push::ProgressNotification, repository::repo, utils,
	},
	ProgressPercent,
};
use crossbeam_channel::Sender;
use git2::{BranchType, FetchOptions, ProxyOptions};
use scopetime::scope_time;
use utils::bytes2string;

pub use callbacks::Callbacks;
pub use tags::tags_missing_remote;

use super::RepoPath;

/// origin
pub const DEFAULT_REMOTE_NAME: &str = "origin";

///
pub fn proxy_auto<'a>() -> ProxyOptions<'a> {
	let mut proxy = ProxyOptions::new();
	proxy.auto();
	proxy
}

///
pub fn get_remotes(repo_path: &RepoPath) -> Result<Vec<String>> {
	scope_time!("get_remotes");

	let repo = repo(repo_path)?;
	let remotes = repo.remotes()?;
	let remotes: Vec<String> =
		remotes.iter().flatten().map(String::from).collect();

	Ok(remotes)
}

/// returns remote name if there is only one single remote
pub fn get_single_remote(repo_path: &RepoPath) -> Result<String> {
	let repo = repo(repo_path)?;
	let remotes = repo.remotes()?;

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

/// returns true if there is only a single remote
pub fn has_single_remote(repo_path: &RepoPath) -> Result<bool> {
	let repo = repo(repo_path)?;
	Ok(repo.remotes()?.len() == 1)
}

///
fn fetch_from_remote(
	repo_path: &RepoPath,
	remote: &str,
	basic_credential: Option<BasicAuthCredential>,
	progress_sender: Option<Sender<ProgressNotification>>,
) -> Result<()> {
	let repo = repo(repo_path)?;

	let mut remote = repo.find_remote(remote)?;

	let mut options = FetchOptions::new();
	let callbacks = Callbacks::new(progress_sender, basic_credential);
	options.prune(git2::FetchPrune::On);
	options.proxy_options(proxy_auto());
	options.download_tags(git2::AutotagOption::All);
	options.remote_callbacks(callbacks.callbacks());
	remote.fetch(&[] as &[&str], Some(&mut options), None)?;
	// fetch tags (also removing remotely deleted ones)
	remote.fetch(
		&["refs/tags/*:refs/tags/*"],
		Some(&mut options),
		None,
	)?;

	Ok(())
}

/// updates/prunes all branches from all remotes
pub fn fetch_all(
	repo_path: &RepoPath,
	basic_credential: &Option<BasicAuthCredential>,
	progress_sender: &Option<Sender<ProgressPercent>>,
) -> Result<()> {
	scope_time!("fetch_all");

	let repo = repo(repo_path)?;
	let remotes = repo
		.remotes()?
		.iter()
		.flatten()
		.map(String::from)
		.collect::<Vec<_>>();
	let remotes_count = remotes.len();

	for (idx, remote) in remotes.into_iter().enumerate() {
		fetch_from_remote(
			repo_path,
			&remote,
			basic_credential.clone(),
			None,
		)?;

		if let Some(sender) = progress_sender {
			let progress = ProgressPercent::new(idx, remotes_count);
			sender.send(progress)?;
		}
	}

	Ok(())
}

/// fetches from upstream/remote for local `branch`
pub(crate) fn fetch(
	repo_path: &RepoPath,
	branch: &str,
	basic_credential: Option<BasicAuthCredential>,
	progress_sender: Option<Sender<ProgressNotification>>,
) -> Result<usize> {
	scope_time!("fetch");

	let repo = repo(repo_path)?;
	let branch_ref = repo
		.find_branch(branch, BranchType::Local)?
		.into_reference();
	let branch_ref = bytes2string(branch_ref.name_bytes())?;
	let remote_name = repo.branch_upstream_remote(&branch_ref)?;
	let remote_name = bytes2string(&remote_name)?;
	let mut remote = repo.find_remote(&remote_name)?;

	let mut options = FetchOptions::new();
	options.download_tags(git2::AutotagOption::All);
	let callbacks = Callbacks::new(progress_sender, basic_credential);
	options.remote_callbacks(callbacks.callbacks());
	options.proxy_options(proxy_auto());

	remote.fetch(&[branch], Some(&mut options), None)?;

	Ok(remote.stats().received_bytes())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::sync::tests::{repo_clone, repo_init};

	#[test]
	fn test_smoke() {
		let (remote_dir, _remote) = repo_init().unwrap();
		let remote_path = remote_dir.path().to_str().unwrap();
		let (repo_dir, _repo) = repo_clone(remote_path).unwrap();
		let repo_path: &RepoPath = &repo_dir
			.into_path()
			.as_os_str()
			.to_str()
			.unwrap()
			.into();

		let remotes = get_remotes(repo_path).unwrap();

		assert_eq!(remotes, vec![String::from("origin")]);

		fetch(repo_path, "master", None, None).unwrap();
	}
}
