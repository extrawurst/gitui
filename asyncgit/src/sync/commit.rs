use std::fs;

use super::{utils::repo, CommitId};
use crate::{error::Result, sync::utils::get_head_repo};
use git2::{
    Buf, ErrorClass, ErrorCode, ObjectType, Oid, Repository,
    Signature,
};
use gpgme::{Context, Protocol};
use scopetime::scope_time;

///
pub fn amend(
    repo_path: &str,
    id: CommitId,
    msg: &str,
) -> Result<CommitId> {
    scope_time!("amend");

    let repo = repo(repo_path)?;
    let commit = repo.find_commit(id.into())?;

    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let parents = commit.parents().collect::<Vec<_>>();
    let parents = parents.iter().collect::<Vec<_>>();

    if let Some(parent) = parents.first() {
        repo.set_head_detached(parent.id())?;
    }

    let commit_id = if sign_enabled(&repo)? {
        let buffer = repo.commit_create_buffer(
            &commit.author(),
            &commit.committer(),
            msg,
            &tree,
            &parents,
        )?;

        let signature = sign(&repo, &buffer)?;

        repo.commit_signed(
            &String::from_utf8(buffer.to_vec())?,
            &signature,
            None,
        )?
    } else {
        repo.commit(
            None,
            &commit.author(),
            &commit.committer(),
            msg,
            &tree,
            &parents,
        )?
    };

    update_head(&repo, commit_id, " (amend)")?;

    Ok(commit_id.into())
}

/// Wrap `Repository::signature` to allow unknown user.name.
///
/// See <https://github.com/extrawurst/gitui/issues/79>.
#[allow(clippy::redundant_pub_crate)]
pub(crate) fn signature_allow_undefined_name(
    repo: &Repository,
) -> std::result::Result<Signature<'_>, git2::Error> {
    let signature = repo.signature();

    if let Err(ref e) = signature {
        if e.code() == ErrorCode::NotFound {
            let config = repo.config()?;

            if let (Err(_), Ok(email_entry)) = (
                config.get_entry("user.name"),
                config.get_entry("user.email"),
            ) {
                if let Some(email) = email_entry.value() {
                    return Signature::now("unknown", email);
                }
            };
        }
    }

    signature
}

/// this does not run any git hooks
pub fn commit(repo_path: &str, msg: &str) -> Result<CommitId> {
    scope_time!("commit");

    let repo = repo(repo_path)?;

    let signature = signature_allow_undefined_name(&repo)?;
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let parents = if let Ok(id) = get_head_repo(&repo) {
        vec![repo.find_commit(id.into())?]
    } else {
        Vec::new()
    };

    let parents = parents.iter().collect::<Vec<_>>();

    let commit_id = if sign_enabled(&repo)? {
        let buffer = repo.commit_create_buffer(
            &signature,
            &signature,
            msg,
            &tree,
            parents.as_slice(),
        )?;
        let signature = sign(&repo, &buffer)?;

        repo.commit_signed(
            &String::from_utf8(buffer.to_vec())?,
            &signature,
            None,
        )?
    } else {
        repo.commit(
            None,
            &signature,
            &signature,
            msg,
            &tree,
            parents.as_slice(),
        )?
    };

    update_head(&repo, commit_id, "")?;

    Ok(commit_id.into())
}

/// Tag a commit.
///
/// This function will return an `Err(…)` variant if the tag’s name is refused
/// by git or if the tag already exists.
pub fn tag(
    repo_path: &str,
    commit_id: &CommitId,
    tag: &str,
) -> Result<CommitId> {
    scope_time!("tag");

    let repo = repo(repo_path)?;

    let signature = signature_allow_undefined_name(&repo)?;
    let object_id = commit_id.get_oid();
    let target =
        repo.find_object(object_id, Some(ObjectType::Commit))?;

    Ok(repo.tag(tag, &target, &signature, "", false)?.into())
}

/// Sign a commit with [`gpgme`].
fn sign(repo: &Repository, buffer: &Buf) -> Result<String> {
    let mut context = Context::from_protocol(Protocol::OpenPgp)?;
    context.set_armor(true);

    if let Ok(signing_key) = repo
        .config()
        .and_then(|cfg| cfg.get_string("user.signingkey"))
    {
        let key = context.get_secret_key(signing_key)?;
        context.add_signer(&key)?;
    }

    let mut signature = Vec::new();

    context.sign_detached(buffer.as_ref(), &mut signature)?;

    String::from_utf8(signature).map_err(Into::into)
}

/// Check whether commit signing is enabled in the Git config. This copes for the case where the
/// config entry is missing, which is not considered an error but instead counts as signing being
/// disabled.
fn sign_enabled(repo: &Repository) -> Result<bool> {
    match repo.config()?.get_bool("commit.gpgsign") {
        Ok(value) => Ok(value),
        Err(e)
            if e.class() == ErrorClass::Config
                && e.code() == ErrorCode::NotFound =>
        {
            Ok(false)
        }
        Err(e) => Err(e.into()),
    }
}

/// Update the current HEAD, and the reference it's pointing to, to the give commit. Based on the
/// state of the repository the current head is either:
///
/// - Available through and **resolvable** through means of git directly.
/// - Extracted from the `.git/HEAD` file in case the current HEAD can't be resolved.
///
/// The HEAD can usually be resolved but in case the repository is fresh, meaning it doesn't have
/// any commits so far, it can't because the reference HEAD is pointing to doesn't exist yet.
///
/// Unfortunately, the [`git2`] crate doesn't provide a way to retrieve the current HEAD without
/// resolving it and the data must be extracted manually from the repo data.
fn update_head(
    repo: &Repository,
    commit_id: Oid,
    commit_type: &str,
) -> Result<()> {
    let head_name = match repo.head() {
        Ok(r) => r.name().unwrap_or_default().to_owned(),
        Err(e)
            if e.class() == ErrorClass::Reference
                && e.code() == ErrorCode::UnbornBranch =>
        {
            // TODO: Check for the possible formats that can be present in the HEAD file and
            // make sure we really get the typical `refs/heads/main` reference that we expect or
            // fail otherwise.
            let head = fs::read_to_string(repo.path().join("HEAD"))?;
            head.strip_prefix("ref:")
                .unwrap_or(&head)
                .trim()
                .to_owned()
        }
        Err(e) => return Err(e.into()),
    };
    let reflog_msg = format!(
        "commit{}: {}",
        commit_type,
        repo.find_commit(commit_id)?.summary().unwrap_or_default()
    );

    let new_head =
        repo.reference(&head_name, commit_id, true, &reflog_msg)?;

    repo.set_head(new_head.name().unwrap_or_default())
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use crate::error::Result;
    use crate::sync::{
        commit, get_commit_details, get_commit_files, stage_add_file,
        tags::get_tags,
        tests::{get_statuses, repo_init, repo_init_empty},
        utils::get_head,
        LogWalker,
    };
    use commit::{amend, tag};
    use git2::Repository;
    use std::{fs::File, io::Write, path::Path};

    fn count_commits(repo: &Repository, max: usize) -> usize {
        let mut items = Vec::new();
        let mut walk = LogWalker::new(&repo, max).unwrap();
        walk.read(&mut items).unwrap();
        items.len()
    }

    #[test]
    fn test_commit() {
        let file_path = Path::new("foo");
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))
            .unwrap()
            .write_all(b"test\nfoo")
            .unwrap();

        assert_eq!(get_statuses(repo_path), (1, 0));

        stage_add_file(repo_path, file_path).unwrap();

        assert_eq!(get_statuses(repo_path), (0, 1));

        commit(repo_path, "commit msg").unwrap();

        assert_eq!(get_statuses(repo_path), (0, 0));
    }

    #[test]
    fn test_commit_in_empty_repo() {
        let file_path = Path::new("foo");
        let (_td, repo) = repo_init_empty().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        assert_eq!(get_statuses(repo_path), (0, 0));

        File::create(&root.join(file_path))
            .unwrap()
            .write_all(b"test\nfoo")
            .unwrap();

        assert_eq!(get_statuses(repo_path), (1, 0));

        stage_add_file(repo_path, file_path).unwrap();

        assert_eq!(get_statuses(repo_path), (0, 1));

        commit(repo_path, "commit msg").unwrap();

        assert_eq!(get_statuses(repo_path), (0, 0));
    }

    #[test]
    fn test_amend() -> Result<()> {
        let file_path1 = Path::new("foo");
        let file_path2 = Path::new("foo2");
        let (_td, repo) = repo_init_empty()?;
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path1))?.write_all(b"test1")?;

        stage_add_file(repo_path, file_path1)?;
        let id = commit(repo_path, "commit msg")?;

        assert_eq!(count_commits(&repo, 10), 1);

        File::create(&root.join(file_path2))?.write_all(b"test2")?;

        stage_add_file(repo_path, file_path2)?;

        let new_id = amend(repo_path, id, "amended")?;

        assert_eq!(count_commits(&repo, 10), 1);

        let details = get_commit_details(repo_path, new_id)?;
        assert_eq!(details.message.unwrap().subject, "amended");

        let files = get_commit_files(repo_path, new_id)?;

        assert_eq!(files.len(), 2);

        let head = get_head(repo_path)?;

        assert_eq!(head, new_id);

        Ok(())
    }

    #[test]
    fn test_tag() -> Result<()> {
        let file_path = Path::new("foo");
        let (_td, repo) = repo_init_empty().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))?
            .write_all(b"test\nfoo")?;

        stage_add_file(repo_path, file_path)?;

        let new_id = commit(repo_path, "commit msg")?;

        tag(repo_path, &new_id, "tag")?;

        assert_eq!(
            get_tags(repo_path).unwrap()[&new_id],
            vec!["tag"]
        );

        assert!(matches!(tag(repo_path, &new_id, "tag"), Err(_)));

        assert_eq!(
            get_tags(repo_path).unwrap()[&new_id],
            vec!["tag"]
        );

        tag(repo_path, &new_id, "second-tag")?;

        assert_eq!(
            get_tags(repo_path).unwrap()[&new_id],
            vec!["second-tag", "tag"]
        );

        Ok(())
    }

    /// Beware: this test has to be run with a `$HOME/.gitconfig` that has
    /// `user.email` not set. Otherwise, git falls back to the value of
    /// `user.email` in `$HOME/.gitconfig` and this test fails.
    ///
    /// As of February 2021, `repo_init_empty` sets all git config locations
    /// to an empty temporary directory, so this constraint is met.
    #[test]
    fn test_empty_email() -> Result<()> {
        let file_path = Path::new("foo");
        let (_td, repo) = repo_init_empty().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))?
            .write_all(b"test\nfoo")?;

        stage_add_file(repo_path, file_path)?;

        repo.config()?.remove("user.email")?;

        let error = commit(repo_path, "commit msg");

        assert!(matches!(error, Err(_)));

        repo.config()?.set_str("user.email", "email")?;

        let success = commit(repo_path, "commit msg");

        assert!(matches!(success, Ok(_)));
        assert_eq!(count_commits(&repo, 10), 1);

        let details =
            get_commit_details(repo_path, success.unwrap()).unwrap();

        assert_eq!(details.author.name, "name");
        assert_eq!(details.author.email, "email");

        Ok(())
    }

    /// See comment to `test_empty_email`.
    #[test]
    fn test_empty_name() -> Result<()> {
        let file_path = Path::new("foo");
        let (_td, repo) = repo_init_empty().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))?
            .write_all(b"test\nfoo")?;

        stage_add_file(repo_path, file_path)?;

        repo.config()?.remove("user.name")?;

        let mut success = commit(repo_path, "commit msg");

        assert!(matches!(success, Ok(_)));
        assert_eq!(count_commits(&repo, 10), 1);

        let mut details =
            get_commit_details(repo_path, success.unwrap()).unwrap();

        assert_eq!(details.author.name, "unknown");
        assert_eq!(details.author.email, "email");

        repo.config()?.set_str("user.name", "name")?;

        success = commit(repo_path, "commit msg");

        assert!(matches!(success, Ok(_)));
        assert_eq!(count_commits(&repo, 10), 2);

        details =
            get_commit_details(repo_path, success.unwrap()).unwrap();

        assert_eq!(details.author.name, "name");
        assert_eq!(details.author.email, "email");

        Ok(())
    }
}
