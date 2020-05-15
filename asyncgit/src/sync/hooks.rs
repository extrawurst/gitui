use crate::error::{Error, Result};
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
) -> Result<HookResult> {
    scope_time!("hooks_commit_msg");

    if hook_runable(repo_path, HOOK_COMMIT_MSG) {
        let mut file = NamedTempFile::new()?;

        write!(file, "{}", msg)?;

        let file_path = file.path().to_str().ok_or_else(|| {
            Error::Generic("can't get temp file's path".to_string())
        })?;

        let res =
            run_hook(repo_path, HOOK_COMMIT_MSG, &[&file_path])?;

        // load possibly altered msg
        let mut file = file.reopen()?;
        msg.clear();
        file.read_to_string(msg)?;

        Ok(res)
    } else {
        Ok(HookResult::Ok)
    }
}

///
pub fn hooks_post_commit(repo_path: &str) -> Result<HookResult> {
    scope_time!("hooks_post_commit");

    if hook_runable(repo_path, HOOK_POST_COMMIT) {
        Ok(run_hook(repo_path, HOOK_POST_COMMIT, &[])?)
    } else {
        Ok(HookResult::Ok)
    }
}

fn hook_runable(path: &str, hook: &str) -> bool {
    let path = Path::new(path);
    let path = path.join(hook);

    path.exists() && path.is_executable()
}

///
#[derive(Debug, PartialEq)]
pub enum HookResult {
    /// Everything went fine
    Ok,
    /// Hook returned error
    NotOk(String),
}

fn run_hook(
    path: &str,
    cmd: &str,
    args: &[&str],
) -> Result<HookResult> {
    let output =
        Command::new(cmd).args(args).current_dir(path).output();

    let output = output.expect("general hook error");

    if output.status.success() {
        Ok(HookResult::Ok)
    } else {
        let err = String::from_utf8(output.stderr)?;
        let out = String::from_utf8(output.stdout)?;
        let formatted = format!("{}{}", out, err);

        Ok(HookResult::NotOk(formatted))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::tests::repo_init;
    use std::fs::File;

    #[test]
    fn test_smoke() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        let mut msg = String::from("test");
        let res = hooks_commit_msg(repo_path, &mut msg).unwrap();

        assert_eq!(res, HookResult::Ok);

        let res = hooks_post_commit(repo_path).unwrap();

        assert_eq!(res, HookResult::Ok);
    }

    fn create_hook(path: &Path, hook_path: &str, hook_script: &[u8]) {
        File::create(&path.join(hook_path))
            .unwrap()
            .write_all(hook_script)
            .unwrap();

        Command::new("chmod")
            .args(&["+x", hook_path])
            .current_dir(path)
            .output()
            .unwrap();
    }

    #[test]
    #[cfg(not(windows))]
    fn test_hooks_commit_msg_ok() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        let hook = b"
#!/bin/sh
exit 0
        ";

        create_hook(root, HOOK_COMMIT_MSG, hook);

        let mut msg = String::from("test");
        let res = hooks_commit_msg(repo_path, &mut msg).unwrap();

        assert_eq!(res, HookResult::Ok);

        assert_eq!(msg, String::from("test"));
    }

    #[test]
    #[cfg(not(windows))]
    fn test_hooks_commit_msg() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        let hook = b"
#!/bin/sh
echo 'msg' > $1
echo 'rejected'
exit 1
        ";

        create_hook(root, HOOK_COMMIT_MSG, hook);

        let mut msg = String::from("test");
        let res = hooks_commit_msg(repo_path, &mut msg).unwrap();

        assert_eq!(
            res,
            HookResult::NotOk(String::from("rejected\n"))
        );

        assert_eq!(msg, String::from("msg\n"));
    }

    #[test]
    #[cfg(not(windows))]
    fn test_commit_msg_no_block_but_alter() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        let hook = b"
#!/bin/sh
echo 'msg' > $1
exit 0
        ";

        create_hook(root, HOOK_COMMIT_MSG, hook);

        let mut msg = String::from("test");
        let res = hooks_commit_msg(repo_path, &mut msg).unwrap();

        assert_eq!(res, HookResult::Ok);
        assert_eq!(msg, String::from("msg\n"));
    }
}
