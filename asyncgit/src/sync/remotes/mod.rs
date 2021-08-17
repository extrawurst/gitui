//!

mod callbacks;
pub(crate) mod push;
pub(crate) mod tags;

use crate::{
	error::{Error, Result},
	sync::{
		cred::BasicAuthCredential,
		remotes::push::ProgressNotification, utils,
	},
};
use crossbeam_channel::Sender;
use git2::{BranchType, FetchOptions, Repository};
use scopetime::scope_time;
use utils::bytes2string;

pub use callbacks::Callbacks;
pub use tags::tags_missing_remote;

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

/// fetches from upstream/remote for `branch`
pub(crate) fn fetch(
	repo_path: &str,
	branch: &str,
	basic_credential: Option<BasicAuthCredential>,
	progress_sender: Option<Sender<ProgressNotification>>,
) -> Result<usize> {
	scope_time!("fetch_origin");

	let repo = utils::repo(repo_path)?;
	let branch_ref = repo
		.find_branch(branch, BranchType::Local)?
		.into_reference();
	let branch_ref = bytes2string(branch_ref.name_bytes())?;
	let remote_name = repo.branch_upstream_remote(&branch_ref)?;
	let remote_name = bytes2string(&*remote_name)?;
	let mut remote = repo.find_remote(&remote_name)?;

	let mut options = FetchOptions::new();
	let callbacks = Callbacks::new(progress_sender, basic_credential);
	options.remote_callbacks(callbacks.callbacks());

	remote.fetch(&[branch], Some(&mut options), None)?;

	Ok(remote.stats().received_bytes())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::sync::tests::{
		debug_cmd_print, repo_clone, repo_init,
	};

	#[test]
	fn test_smoke() {
		let (remote_dir, _remote) = repo_init().unwrap();
		let remote_path = remote_dir.path().to_str().unwrap();
		let (repo_dir, _repo) = repo_clone(remote_path).unwrap();
		let repo_path = repo_dir.path().as_os_str().to_str().unwrap();

		let remotes = get_remotes(repo_path).unwrap();

		assert_eq!(remotes, vec![String::from("origin")]);

		fetch(repo_path, "master", None, None).unwrap();
	}

	#[test]
	fn test_default_remote() {
		let (remote_dir, _remote) = repo_init().unwrap();
		let remote_path = remote_dir.path().to_str().unwrap();
		let (repo_dir, _repo) = repo_clone(remote_path).unwrap();
		let repo_path = repo_dir.path().as_os_str().to_str().unwrap();

		debug_cmd_print(
			repo_path,
			&format!("git remote add second {}", remote_path)[..],
		);

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
		let (remote_dir, _remote) = repo_init().unwrap();
		let remote_path = remote_dir.path().to_str().unwrap();
		let (repo_dir, _repo) = repo_clone(remote_path).unwrap();
		let repo_path = repo_dir.path().as_os_str().to_str().unwrap();

		debug_cmd_print(
			repo_path,
			"git remote rename origin alternate",
		);

		debug_cmd_print(
			repo_path,
			&format!("git remote add origin {}", remote_path)[..],
		);

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
		let (remote_dir, _remote) = repo_init().unwrap();
		let remote_path = remote_dir.path().to_str().unwrap();
		let (repo_dir, _repo) = repo_clone(remote_path).unwrap();
		let repo_path = repo_dir.path().as_os_str().to_str().unwrap();

		debug_cmd_print(
			repo_path,
			"git remote rename origin alternate",
		);

		debug_cmd_print(
			repo_path,
			&format!("git remote add someremote {}", remote_path)[..],
		);

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
}
