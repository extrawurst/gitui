//! sync git api

mod branch;
mod commit;
mod commit_details;
mod commit_files;
mod commits_info;
pub mod cred;
pub mod diff;
mod hooks;
mod hunks;
mod ignore;
mod logwalker;
mod remotes;
mod reset;
mod stash;
pub mod status;
mod tags;
pub mod utils;

pub(crate) use branch::get_branch_name;
pub use branch::{
    checkout_branch, create_branch, delete_branch,
    get_branches_to_display, rename_branch, BranchForDisplay,
};
pub use commit::{amend, commit, tag};
pub use commit_details::{
    get_commit_details, CommitDetails, CommitMessage,
};
pub use commit_files::get_commit_files;
pub use commits_info::{get_commits_info, CommitId, CommitInfo};
pub use diff::get_diff_commit;
pub use hooks::{hooks_commit_msg, hooks_post_commit, HookResult};
pub use hunks::{reset_hunk, stage_hunk, unstage_hunk};
pub use ignore::add_to_ignore;
pub use logwalker::LogWalker;
pub use remotes::{
    fetch_origin, get_remotes, push, ProgressNotification,
};
pub use reset::{reset_stage, reset_workdir};
pub use stash::{get_stashes, stash_apply, stash_drop, stash_save};
pub use tags::{get_tags, CommitTags, Tags};
pub use utils::{
    get_head, get_head_tuple, is_bare_repo, is_repo, stage_add_all,
    stage_add_file, stage_addremoved, Head,
};

#[cfg(test)]
mod tests {
    use super::status::{get_status, StatusType};
    use crate::error::Result;
    use git2::Repository;
    use std::process::Command;
    use tempfile::TempDir;

    ///
    pub fn repo_init_empty() -> Result<(TempDir, Repository)> {
        let td = TempDir::new()?;
        let repo = Repository::init(td.path())?;
        {
            let mut config = repo.config()?;
            config.set_str("user.name", "name")?;
            config.set_str("user.email", "email")?;
        }
        Ok((td, repo))
    }

    ///
    pub fn repo_init() -> Result<(TempDir, Repository)> {
        let td = TempDir::new()?;
        let repo = Repository::init(td.path())?;
        {
            let mut config = repo.config()?;
            config.set_str("user.name", "name")?;
            config.set_str("user.email", "email")?;

            let mut index = repo.index()?;
            let id = index.write_tree()?;

            let tree = repo.find_tree(id)?;
            let sig = repo.signature()?;
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                "initial",
                &tree,
                &[],
            )?;
        }
        Ok((td, repo))
    }

    /// helper returning amount of files with changes in the (wd,stage)
    pub fn get_statuses(repo_path: &str) -> (usize, usize) {
        (
            get_status(repo_path, StatusType::WorkingDir, true)
                .unwrap()
                .len(),
            get_status(repo_path, StatusType::Stage, true)
                .unwrap()
                .len(),
        )
    }

    ///
    pub fn debug_cmd_print(path: &str, cmd: &str) {
        let cmd = debug_cmd(path, cmd);
        eprintln!("\n----\n{}", cmd);
    }

    fn debug_cmd(path: &str, cmd: &str) -> String {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(&["/C", cmd])
                .current_dir(path)
                .output()
                .unwrap()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(path)
                .output()
                .unwrap()
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        format!(
            "{}{}",
            if stdout.is_empty() {
                String::new()
            } else {
                format!("out:\n{}", stdout)
            },
            if stderr.is_empty() {
                String::new()
            } else {
                format!("err:\n{}", stderr)
            }
        )
    }
}
