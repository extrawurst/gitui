use crate::sync::repository::repo;
use scopetime::scope_time;
use crate::error::Result;

use super::RepoPath;


//pub struct WorkTree {
//	pub name: String,
//}

/// TODO: Do stuff
pub fn worktrees(
	repo_path: &RepoPath,
) -> Result<Vec<String>> {
	scope_time!("tree_files");

    log::trace!("Trying to print worktrees");
	let repo = repo(repo_path)?;
    log::trace!("Is worktree: {}", repo.is_worktree());
    for w in repo.worktrees()?.iter() {
        log::trace!("{}", w.unwrap());
    };
    Ok(repo.worktrees()?.iter().map(|s| s.unwrap().to_string()).collect())
}
