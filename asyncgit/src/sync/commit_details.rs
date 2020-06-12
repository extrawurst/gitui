use super::{utils::repo, CommitId};
use crate::error::Result;
use git2::Signature;
use scopetime::scope_time;

///
#[derive(Debug, PartialEq)]
pub struct CommitSignature {
    ///
    pub name: String,
    ///
    pub email: String,
    /// time in secs since Unix epoch
    pub time: i64,
}

impl CommitSignature {
    /// convert from git2-rs `Signature`
    pub fn from(s: Signature<'_>) -> Self {
        Self {
            name: s.name().unwrap_or("").to_string(),
            email: s.email().unwrap_or("").to_string(),

            time: s.when().seconds(),
        }
    }
}

///
pub struct CommitMessage {
    /// first line
    pub subject: String,
    /// remaining lines if more than one
    pub body: Option<String>,
}

impl CommitMessage {
    pub fn from(s: &str) -> Self {
        if let Some(idx) = s.find('\n') {
            let (first, rest) = s.split_at(idx);
            Self {
                subject: first.to_string(),
                body: if rest.is_empty() {
                    None
                } else {
                    Some(rest.to_string())
                },
            }
        } else {
            Self {
                subject: s.to_string(),
                body: None,
            }
        }
    }

    ///
    pub fn combine(self) -> String {
        if let Some(body) = self.body {
            format!("{}{}", self.subject, body)
        } else {
            self.subject
        }
    }
}

///
pub struct CommitDetails {
    ///
    pub author: CommitSignature,
    /// committer when differs to `author` otherwise None
    pub committer: Option<CommitSignature>,
    ///
    pub message: Option<CommitMessage>,
    ///
    pub hash: String,
}

///
pub fn get_commit_details(
    repo_path: &str,
    id: CommitId,
) -> Result<CommitDetails> {
    scope_time!("get_commit_details");

    let repo = repo(repo_path)?;

    let commit = repo.find_commit(id.into())?;

    let author = CommitSignature::from(commit.author());
    let committer = CommitSignature::from(commit.committer());
    let committer = if author == committer {
        None
    } else {
        Some(committer)
    };

    let message = commit.message().map(|m| CommitMessage::from(m));

    let details = CommitDetails {
        author,
        committer,
        message,
        hash: id.to_string(),
    };

    Ok(details)
}
