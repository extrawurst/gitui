//! sync git api

pub mod diff;
mod hooks;
mod hunks;
mod reset;
pub mod status;
pub mod utils;

pub use hooks::{hooks_commit_msg, hooks_post_commit, HookResult};
pub use hunks::{stage_hunk, unstage_hunk};
pub use reset::{reset_stage, reset_workdir};
pub use utils::{commit, stage_add};

#[cfg(test)]
mod tests {
    use git2::Repository;
    use std::process::Command;
    use tempfile::TempDir;

    ///
    pub fn repo_init_empty() -> (TempDir, Repository) {
        let td = TempDir::new().unwrap();
        let repo = Repository::init(td.path()).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "name").unwrap();
            config.set_str("user.email", "email").unwrap();
        }
        (td, repo)
    }

    pub fn repo_init() -> (TempDir, Repository) {
        let td = TempDir::new().unwrap();
        let repo = Repository::init(td.path()).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "name").unwrap();
            config.set_str("user.email", "email").unwrap();

            let mut index = repo.index().unwrap();
            let id = index.write_tree().unwrap();

            let tree = repo.find_tree(id).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                "initial",
                &tree,
                &[],
            )
            .unwrap();
        }
        (td, repo)
    }

    ///
    pub fn debug_cmd_print(path: &str, cmd: &str) {
        eprintln!("\n----\n{}", debug_cmd(path, cmd))
    }

    fn debug_cmd(path: &str, cmd: &str) -> String {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(&["/C", cmd])
                .current_dir(path)
                .output()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(path)
                .output()
        };

        let output = output.unwrap();
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
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
