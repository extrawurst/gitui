use crate::sync::repository::repo;
use scopetime::scope_time;
use crate::error::Result;

use super::RepoPath;


/// This should kinda represent a worktree
pub struct WorkTree {
    /// Worktree name (wich is also the folder i think)
	pub name: String,
}

/// TODO: Do stuff
pub fn worktrees(
	repo_path: &RepoPath,
) -> Result<Vec<WorkTree>> {
	scope_time!("worktrees");

	let repo = repo(repo_path)?;

    Ok(repo.worktrees()?
       .iter()
       .map(|s| WorkTree {
           name: s.unwrap().to_string()
       })
       .collect()
    )
}
