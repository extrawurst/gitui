//! credentials git helper

use super::{
	remotes::get_default_remote_in_repo, repository::repo, RepoPath,
};
use crate::error::{Error, Result};
use git2::CredentialHelper;

/// basic Authentication Credentials
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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

	//TODO: look at Cred::credential_helper,
	//if the username is in the url we need to set it here,
	//I dont think `config` will pick it up

	if let Ok(config) = repo.config() {
		helper.config(&config);
	}

	Ok(match helper.execute() {
		Some((username, password)) => {
			BasicAuthCredential::new(Some(username), Some(password))
		}
		None => extract_cred_from_url(&url),
	})
}

/// extract credentials from url
pub fn extract_cred_from_url(url: &str) -> BasicAuthCredential {
	url::Url::parse(url).map_or_else(
		|_| BasicAuthCredential::new(None, None),
		|url| {
			BasicAuthCredential::new(
				if url.username() == "" {
					None
				} else {
					Some(url.username().to_owned())
				},
				url.password().map(std::borrow::ToOwned::to_owned),
			)
		},
	)
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

		need_username_password(repo_path).unwrap();
	}

	#[test]
	#[serial]
	fn test_extract_username_password_from_repo() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

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

		extract_username_password(repo_path).unwrap();
	}
}
