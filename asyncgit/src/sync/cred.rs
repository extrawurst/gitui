//! credentials git helper

use std::process::Command;

use super::{
	remotes::get_default_remote_in_repo, repository::repo, RepoPath,
};
use crate::error::{Error, Result};
use git2::{Config, CredentialHelper};
use scopetime::scope_time;

/// basic Authentication Credentials
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BasicAuthCredential {
	///
	pub username: Option<String>,
	///
	pub password: Option<String>,
}

impl BasicAuthCredential {
	///
	pub const fn is_complete(&self) -> bool {
		self.username.is_some() && self.password.is_some()
	}
	///
	pub const fn new(
		username: Option<String>,
		password: Option<String>,
	) -> Self {
		Self { username, password }
	}
}

/// know if username and password are needed for this url
pub fn need_username_password(repo_path: &RepoPath) -> Result<bool> {
	let repo = repo(repo_path)?;
	let remote =
		repo.find_remote(&get_default_remote_in_repo(&repo)?)?;
	let url = remote
		.pushurl()
		.or_else(|| remote.url())
		.ok_or(Error::UnknownRemote)?
		.to_owned();
	let is_http = url.starts_with("http");
	Ok(is_http)
}

/// extract username and password
pub fn extract_username_password(
	repo_path: &RepoPath,
) -> Result<BasicAuthCredential> {
	let repo = repo(repo_path)?;
	let url = repo
		.find_remote(&get_default_remote_in_repo(&repo)?)?
		.url()
		.ok_or(Error::UnknownRemote)?
		.to_owned();
	let mut helper = CredentialHelper::new(&url);

	if use_credential_store(&repo.config()?) {
		if let Some(cred) = git_credential_fill(&url) {
			return Ok(cred);
		}
	}

	//TODO: look at Cred::credential_helper,
	//if the username is in the url we need to set it here,
	//I dont think `config` will pick it up

	if let Ok(config) = Config::open_default() {
		helper.config(&config);
	}

	Ok(match helper.execute() {
		Some((username, password)) => {
			BasicAuthCredential::new(Some(username), Some(password))
		}
		None => extract_cred_from_url(&url),
	})
}

fn use_credential_store(config: &Config) -> bool {
	config
		.get_entry("credential.helper")
		.ok()
		.as_ref()
		.and_then(git2::ConfigEntry::value)
		.map(|val| val == "store")
		.unwrap_or_default()
}

// tries calling:
// printf "protocol=https\nhost=github.com\n" | git credential fill
// see https://git-scm.com/book/en/v2/Git-Tools-Credential-Storage
// TODO: use input stream
fn git_credential_fill(url: &str) -> Option<BasicAuthCredential> {
	scope_time!("git_credential_fill");

	let url = url::Url::parse(url).ok()?;

	let host = url.domain()?;
	let protocol = url.scheme();

	let cmd = format!("protocol={}\nhost={}\n", protocol, host);
	let cmd = format!("printf \"{}\" | git credential fill", cmd);

	let bash_args = vec!["-c".to_string(), cmd];

	let res = Command::new("bash").args(bash_args).output().ok()?;
	let output = String::from_utf8_lossy(res.stdout.as_slice());

	let mut res = BasicAuthCredential::default();
	for line in output.lines() {
		if let Some(tuple) = split_once(line, "=") {
			if tuple.0 == "username" {
				res.username = Some(tuple.1.to_string());
			} else if tuple.0 == "password" {
				res.password = Some(tuple.1.to_string());
			}
		}
	}

	if res.username.is_some() && res.password.is_some() {
		return Some(res);
	}

	None
}

fn split_once<'a>(
	v: &'a str,
	splitter: &str,
) -> Option<(&'a str, &'a str)> {
	let mut split = v.split(splitter);

	let key = split.next();
	let val = split.next();

	key.zip(val)
}

/// extract credentials from url
pub fn extract_cred_from_url(url: &str) -> BasicAuthCredential {
	if let Ok(url) = url::Url::parse(url) {
		BasicAuthCredential::new(
			if url.username() == "" {
				None
			} else {
				Some(url.username().to_owned())
			},
			url.password().map(std::borrow::ToOwned::to_owned),
		)
	} else {
		BasicAuthCredential::new(None, None)
	}
}

#[cfg(test)]
mod tests {
	use crate::sync::{
		cred::{
			extract_cred_from_url, extract_username_password,
			need_username_password, BasicAuthCredential,
		},
		remotes::DEFAULT_REMOTE_NAME,
		tests::repo_init,
		RepoPath,
	};
	use serial_test::serial;

	#[test]
	fn test_credential_complete() {
		assert_eq!(
			BasicAuthCredential::new(
				Some("username".to_owned()),
				Some("password".to_owned())
			)
			.is_complete(),
			true
		);
	}

	#[test]
	fn test_credential_not_complete() {
		assert_eq!(
			BasicAuthCredential::new(
				None,
				Some("password".to_owned())
			)
			.is_complete(),
			false
		);
		assert_eq!(
			BasicAuthCredential::new(
				Some("username".to_owned()),
				None
			)
			.is_complete(),
			false
		);
		assert_eq!(
			BasicAuthCredential::new(None, None).is_complete(),
			false
		);
	}

	#[test]
	fn test_extract_username_from_url() {
		assert_eq!(
			extract_cred_from_url("https://user@github.com"),
			BasicAuthCredential::new(Some("user".to_owned()), None)
		);
	}

	#[test]
	fn test_extract_username_password_from_url() {
		assert_eq!(
			extract_cred_from_url("https://user:pwd@github.com"),
			BasicAuthCredential::new(
				Some("user".to_owned()),
				Some("pwd".to_owned())
			)
		);
	}

	#[test]
	fn test_extract_nothing_from_url() {
		assert_eq!(
			extract_cred_from_url("https://github.com"),
			BasicAuthCredential::new(None, None)
		);
	}

	#[test]
	#[serial]
	fn test_need_username_password_if_https() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		//TODO:
		// env::set_current_dir(repo_path).unwrap();
		repo.remote(DEFAULT_REMOTE_NAME, "http://user@github.com")
			.unwrap();

		assert_eq!(need_username_password(repo_path).unwrap(), true);
	}

	#[test]
	#[serial]
	fn test_dont_need_username_password_if_ssh() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		//TODO:
		// env::set_current_dir(repo_path).unwrap();
		repo.remote(DEFAULT_REMOTE_NAME, "git@github.com:user/repo")
			.unwrap();

		assert_eq!(need_username_password(repo_path).unwrap(), false);
	}

	#[test]
	#[serial]
	fn test_dont_need_username_password_if_pushurl_ssh() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		repo.remote(DEFAULT_REMOTE_NAME, "http://user@github.com")
			.unwrap();
		repo.remote_set_pushurl(
			DEFAULT_REMOTE_NAME,
			Some("git@github.com:user/repo"),
		)
		.unwrap();

		assert_eq!(need_username_password(repo_path).unwrap(), false);
	}

	#[test]
	#[serial]
	#[should_panic]
	fn test_error_if_no_remote_when_trying_to_retrieve_if_need_username_password(
	) {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		//TODO:
		// env::set_current_dir(repo_path).unwrap();

		need_username_password(repo_path).unwrap();
	}

	#[test]
	#[serial]
	fn test_extract_username_password_from_repo() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		//TODO:
		// env::set_current_dir(repo_path).unwrap();
		repo.remote(
			DEFAULT_REMOTE_NAME,
			"http://user:pass@github.com",
		)
		.unwrap();

		assert_eq!(
			extract_username_password(repo_path).unwrap(),
			BasicAuthCredential::new(
				Some("user".to_owned()),
				Some("pass".to_owned())
			)
		);
	}

	#[test]
	#[serial]
	fn test_extract_username_from_repo() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		//TODO:
		// env::set_current_dir(repo_path).unwrap();
		repo.remote(DEFAULT_REMOTE_NAME, "http://user@github.com")
			.unwrap();

		assert_eq!(
			extract_username_password(repo_path).unwrap(),
			BasicAuthCredential::new(Some("user".to_owned()), None)
		);
	}

	#[test]
	#[serial]
	#[should_panic]
	fn test_error_if_no_remote_when_trying_to_extract_username_password(
	) {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		//TODO: not needed anymore?
		// env::set_current_dir(repo_path).unwrap();

		extract_username_password(repo_path).unwrap();
	}
}
