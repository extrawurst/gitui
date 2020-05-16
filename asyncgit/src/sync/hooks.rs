use crate::error::{Error, Result};
use scopetime::scope_time;
use std::path::PathBuf;
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
            Error::Generic(
                "temp file path contains invalid unicode sequences."
                    .to_string(),
            )
        })?;

        let res = run_hook(repo_path, HOOK_COMMIT_MSG, &[&file_path]);

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
        Ok(run_hook(repo_path, HOOK_POST_COMMIT, &[]))
    } else {
        Ok(HookResult::Ok)
    }
}

fn hook_runable(path: &str, hook: &str) -> bool {
    let path = Path::new(path);
    let path = path.join(hook);

    path.exists() && is_executable(path)
}

///
#[derive(Debug, PartialEq)]
pub enum HookResult {
    /// Everything went fine
    Ok,
    /// Hook returned error
    NotOk(String),
}

fn run_hook(path: &str, cmd: &str, args: &[&str]) -> HookResult {
    let mut bash_args = vec![cmd.to_string()];
    bash_args.extend_from_slice(
        &args
            .iter()
            .map(|x| (*x).to_string())
            .collect::<Vec<String>>(),
    );

    #[cfg(windows)]
    {
        bash_args = bash_args
            .iter()
            .map(|x| map_windows_path_for_bash(x.as_str()))
            .collect();
    }

    print!("running bash with {:?}", bash_args);
    let output = Command::new("bash")
        .args(bash_args.iter().map(|x| x.replace('\\', "/")))
        .current_dir(path)
        .output();

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

        #[cfg(not(windows))]
        {
            Command::new("chmod")
                .args(&["+x", hook_path])
                .current_dir(path)
                .output()
                .unwrap();
        }
    }

    #[test]
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

#[cfg(not(windows))]
fn is_executable(path: PathBuf) -> bool {
    use is_executable::IsExecutable;
    path.is_executable()
}

#[cfg(windows)]
/// windows does not consider bash scripts to be executable so we consider everything
/// to be executable (which is not far from the truth for windows platform.)
fn is_executable(_: PathBuf) -> bool {
    true
}

#[cfg(windows)]
/// git for windows provides a bash implementation to be used along with git.
/// this function maps file paths form windows path form like `C:\Users\Guest` to
/// `/mnt/c/Users/Guest` so that scripts on windows file system can be used from bash.
fn map_windows_path_for_bash(path: &str) -> String {
    let mut chars = path.chars();
    let mapped = match (chars.next(), chars.next(), chars.as_str()) {
        (Some(drive), Some(':'), rest) => format!(
            "/mnt/{}{}",
            drive.to_lowercase(),
            rest.replace('\\', "/")
        ),
        _ => path.to_string(),
    };

    print!("mapped windows path {:?} to {:?}", path, mapped);
    mapped
}
