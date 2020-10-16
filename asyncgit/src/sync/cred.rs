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

/// know if username and password are needed for this url
pub fn need_username_password(remote: &str) -> Result<bool> {
    let repo = crate::sync::utils::repo(CWD)?;
    let url = repo
        .find_remote(remote)?
        .url()
        .ok_or_else(|| Error::Generic("No remote URL".to_owned()))?
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
        .ok_or_else(|| Error::Generic("No remote URL".to_owned()))?
        .to_owned();
    let mut helper = CredentialHelper::new(&url);

    if let Ok(config) = Config::open_default() {
        helper.config(&config);
    }
    Ok(match helper.execute() {
        Some((username, password)) => BasicAuthCredential {
            username: Some(username),
            password: Some(password),
        },
        None => extract_cred_from_url(&url),
    })
}

/// extract credentials from url
pub fn extract_cred_from_url(url: &str) -> BasicAuthCredential {
    if let Ok(url) = url::Url::parse(url) {
        BasicAuthCredential {
            username: if url.username() == "" {
                None
            } else {
                Some(url.username().to_owned())
            },
            password: url.password().map(|pwd| pwd.to_owned()),
        }
    } else {
        BasicAuthCredential {
            username: None,
            password: None,
        }
    }
}
