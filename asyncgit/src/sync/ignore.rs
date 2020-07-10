use super::utils::{repo, work_dir};
use crate::error::Result;
use scopetime::scope_time;
use std::{fs::OpenOptions, io::Write};

static GITIGNORE: &str = ".gitignore";

/// add file or path to root ignore file
pub fn add_to_ignore(
    repo_path: &str,
    path_to_ignore: &str,
) -> Result<()> {
    scope_time!("add_to_ignore");

    let repo = repo(repo_path)?;

    let ignore_file = work_dir(&repo).join(GITIGNORE);

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(ignore_file)?;

    writeln!(file, "{}", path_to_ignore)?;

    Ok(())
}
