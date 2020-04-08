use is_executable::IsExecutable;
use scopetime::scope_time;
use std::{
    io::{Read, Write},
    path::Path,
    process::Command,
};
use tempfile::NamedTempFile;

const HOOK_POST_COMMIT: &str = ".git/hooks/post-commit";
const HOOK_COMMIT_MSG: &str = ".git/hooks/commit-msg";

///
pub fn hooks_commit_msg(
    repo_path: &str,
    msg: &mut String,
) -> HookResult {
    scope_time!("hooks_commit_msg");

    if hook_runable(repo_path, HOOK_COMMIT_MSG) {
        let mut file = NamedTempFile::new().unwrap();

        write!(file, "{}", msg).unwrap();

        let file_path = file.path().to_str().unwrap();

        let res = run_hook(repo_path, HOOK_COMMIT_MSG, &[&file_path]);

        if let HookResult::NotOk(e) = res {
            file.read_to_string(msg).unwrap();
            HookResult::NotOk(e)
        } else {
            HookResult::Ok
        }
    } else {
        HookResult::Ok
    }
}

///
pub fn hooks_post_commit(repo_path: &str) -> HookResult {
    scope_time!("hooks_post_commit");

    if hook_runable(repo_path, HOOK_POST_COMMIT) {
        run_hook(repo_path, HOOK_POST_COMMIT, &[])
    } else {
        HookResult::Ok
    }
}

fn hook_runable(path: &str, hook: &str) -> bool {
    let path = Path::new(path);
    let path = path.join(hook);

    path.exists() && path.is_executable()
}

///
pub enum HookResult {
    /// Everything went fine
    Ok,
    /// Hook returned error
    NotOk(String),
}

fn run_hook(path: &str, cmd: &str, args: &[&str]) -> HookResult {
    let output =
        Command::new(cmd).args(args).current_dir(path).output();

    let output = output.expect("general hook error");

    if output.status.success() {
        HookResult::Ok
    } else {
        let err = String::from_utf8(output.stderr).unwrap();
        let out = String::from_utf8(output.stdout).unwrap();
        let formatted = format!("{}{}", out, err);

        HookResult::NotOk(formatted)
    }
}
