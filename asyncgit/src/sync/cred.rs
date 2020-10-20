//! credentials git helper

use git2::{Config, CredentialHelper};

use crate::error::{Error, Result};
use crate::CWD;

/// basic Authentication Credentials
#[derive(Debug, Clone, Default)]
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
