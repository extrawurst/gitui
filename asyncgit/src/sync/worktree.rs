use crate::sync::{repository::repo, branch::get_branch_name};
use scopetime::scope_time;
use crate::error::Result;

use super::RepoPath;


/// This should kinda represent a worktree
pub struct WorkTree {
    /// Worktree name (wich is also the folder i think)
	pub name: String,
    /// Worktree branch name
    pub branch: String,
}

/// TODO: Do stuff
pub fn worktrees(
	repo_path: &RepoPath,
) -> Result<Vec<WorkTree>> {
	scope_time!("worktrees");

	let repo_obj = repo(repo_path)?;

    let result: Vec<WorkTree> = repo_obj.worktrees()?
       .iter()
       .map(|s| WorkTree {
           name: s.unwrap().to_string(),
           branch: worktree_branch(s.unwrap(), repo_path).unwrap(),
       })
       .collect();
    Ok(result)
}

fn worktree_branch(name: &str, repo_path: &RepoPath) -> Result<String> {
	let repo_obj = repo(repo_path)?;
    let worktree = repo_obj.find_worktree(name)?;
    let worktree_path = RepoPath::from(worktree.path().to_str().unwrap());
    get_branch_name(&worktree_path)
}

