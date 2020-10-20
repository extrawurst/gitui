//! credentials git helper

use git2::{Config, CredentialHelper};

use crate::error::{Error, Result};
use crate::CWD;

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
    pub fn is_complete(&self) -> bool {
        self.username.is_some() && self.password.is_some()
    }
    ///
    pub fn new(
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        BasicAuthCredential { username, password }
    }
}

/// know if username and password are needed for this url
pub fn need_username_password(remote: &str) -> Result<bool> {
    let repo = crate::sync::utils::repo(CWD)?;
    let url = repo
        .find_remote(remote)?
        .url()
        .ok_or(Error::NoRemote)?
        .to_owned();
    let is_http = url.starts_with("http");
    Ok(is_http)
}

/// extract username and password
pub fn extract_username_password(
    remote: &str,
) -> Result<BasicAuthCredential> {
    let repo = crate::sync::utils::repo(CWD)?;
    let url = repo
        .find_remote(remote)?
        .url()
        .ok_or(Error::NoRemote)?
        .to_owned();
    let mut helper = CredentialHelper::new(&url);

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

/// extract credentials from url
pub fn extract_cred_from_url(url: &str) -> BasicAuthCredential {
    if let Ok(url) = url::Url::parse(url) {
        BasicAuthCredential::new(
            if url.username() == "" {
                None
            } else {
                Some(url.username().to_owned())
            },
            url.password().map(|pwd| pwd.to_owned()),
        )
    } else {
        BasicAuthCredential::new(None, None)
    }
}

#[cfg(test)]
mod tests {
    use crate::sync::cred::{
        extract_cred_from_url, extract_username_password,
        need_username_password, BasicAuthCredential,
    };
    use crate::sync::tests::repo_init;
    use crate::sync::DEFAULT_REMOTE_NAME;
    use serial_test::serial;
    use std::env;

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
        let repo_path = root.as_os_str().to_str().unwrap();

        env::set_current_dir(repo_path).unwrap();
        repo.remote(DEFAULT_REMOTE_NAME, "http://user@github.com")
            .unwrap();

        assert_eq!(
            need_username_password(DEFAULT_REMOTE_NAME).unwrap(),
            true
        );
    }

    #[test]
    #[serial]
    fn test_dont_need_username_password_if_ssh() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        env::set_current_dir(repo_path).unwrap();
        repo.remote(DEFAULT_REMOTE_NAME, "git@github.com:user/repo")
            .unwrap();

        assert_eq!(
            need_username_password(DEFAULT_REMOTE_NAME).unwrap(),
            false
        );
    }

    #[test]
    #[serial]
    #[should_panic]
    fn test_error_if_no_remote_when_trying_to_retrieve_if_need_username_password(
    ) {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        env::set_current_dir(repo_path).unwrap();

        need_username_password(DEFAULT_REMOTE_NAME).unwrap();
    }

    #[test]
    #[serial]
    fn test_extract_username_password_from_repo() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        env::set_current_dir(repo_path).unwrap();
        repo.remote(
            DEFAULT_REMOTE_NAME,
            "http://user:pass@github.com",
        )
        .unwrap();

        assert_eq!(
            extract_username_password(DEFAULT_REMOTE_NAME).unwrap(),
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
        let repo_path = root.as_os_str().to_str().unwrap();

        env::set_current_dir(repo_path).unwrap();
        repo.remote(DEFAULT_REMOTE_NAME, "http://user@github.com")
            .unwrap();

        assert_eq!(
            extract_username_password(DEFAULT_REMOTE_NAME).unwrap(),
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
        let repo_path = root.as_os_str().to_str().unwrap();

        env::set_current_dir(repo_path).unwrap();

        extract_username_password(DEFAULT_REMOTE_NAME).unwrap();
    }
}
