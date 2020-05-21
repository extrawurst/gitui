//! sync git api

mod commits_info;
pub mod diff;
mod hooks;
mod hunks;
mod logwalker;
mod reset;
mod stash;
pub mod status;
mod tags;
pub mod utils;

pub use commits_info::{get_commits_info, CommitInfo};
pub use hooks::{hooks_commit_msg, hooks_post_commit, HookResult};
pub use hunks::{stage_hunk, unstage_hunk};
pub use logwalker::LogWalker;
pub use reset::{
    reset_stage, reset_workdir_file, reset_workdir_folder,
};
pub use stash::stash_save;
pub use tags::{get_tags, Tags};
pub use utils::{
    commit, stage_add_all, stage_add_file, stage_addremoved,
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
            get_status(repo_path, StatusType::WorkingDir)
                .unwrap()
                .len(),
            get_status(repo_path, StatusType::Stage).unwrap().len(),
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
