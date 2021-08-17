use super::utils::{repo, work_dir};
use crate::error::{Error, Result};
use scopetime::scope_time;
use std::{
	fs::File,
	io::{Read, Write},
	path::Path,
	process::Command,
};

const HOOK_POST_COMMIT: &str = ".git/hooks/post-commit";
const HOOK_PRE_COMMIT: &str = ".git/hooks/pre-commit";
const HOOK_COMMIT_MSG: &str = ".git/hooks/commit-msg";
const HOOK_COMMIT_MSG_TEMP_FILE: &str = ".git/COMMIT_EDITMSG";

/// this hook is documented here <https://git-scm.com/docs/githooks#_commit_msg>
/// we use the same convention as other git clients to create a temp file containing
/// the commit message at `.git/COMMIT_EDITMSG` and pass it's relative path as the only
/// parameter to the hook script.
pub fn hooks_commit_msg(
	repo_path: &str,
	msg: &mut String,
) -> Result<HookResult> {
	scope_time!("hooks_commit_msg");

	let work_dir = work_dir_as_string(repo_path)?;

	if hook_runable(work_dir.as_str(), HOOK_COMMIT_MSG) {
		let temp_file = Path::new(work_dir.as_str())
			.join(HOOK_COMMIT_MSG_TEMP_FILE);
		File::create(&temp_file)?.write_all(msg.as_bytes())?;

		let res = run_hook(
			work_dir.as_str(),
			HOOK_COMMIT_MSG,
			&[HOOK_COMMIT_MSG_TEMP_FILE],
		)?;

		// load possibly altered msg
		msg.clear();
		File::open(temp_file)?.read_to_string(msg)?;

		Ok(res)
	} else {
		Ok(HookResult::Ok)
	}
}

/// this hook is documented here <https://git-scm.com/docs/githooks#_pre_commit>
///
pub fn hooks_pre_commit(repo_path: &str) -> Result<HookResult> {
	scope_time!("hooks_pre_commit");

	let work_dir = work_dir_as_string(repo_path)?;

	if hook_runable(work_dir.as_str(), HOOK_PRE_COMMIT) {
		Ok(run_hook(work_dir.as_str(), HOOK_PRE_COMMIT, &[])?)
	} else {
		Ok(HookResult::Ok)
	}
}
///
pub fn hooks_post_commit(repo_path: &str) -> Result<HookResult> {
	scope_time!("hooks_post_commit");

	let work_dir = work_dir_as_string(repo_path)?;
	let work_dir_str = work_dir.as_str();

	if hook_runable(work_dir_str, HOOK_POST_COMMIT) {
		Ok(run_hook(work_dir_str, HOOK_POST_COMMIT, &[])?)
	} else {
		Ok(HookResult::Ok)
	}
}

fn work_dir_as_string(repo_path: &str) -> Result<String> {
	let repo = repo(repo_path)?;
	work_dir(&repo)?
		.to_str()
		.map(std::string::ToString::to_string)
		.ok_or_else(|| {
			Error::Generic(
				"workdir contains invalid utf8".to_string(),
			)
		})
}

fn hook_runable(path: &str, hook: &str) -> bool {
	let path = Path::new(path);
	let path = path.join(hook);

	path.exists() && is_executable(&path)
}

///
#[derive(Debug, PartialEq)]
pub enum HookResult {
	/// Everything went fine
	Ok,
	/// Hook returned error
	NotOk(String),
}

/// this function calls hook scripts based on conventions documented here
/// see <https://git-scm.com/docs/githooks>
fn run_hook(
	path: &str,
	hook_script: &str,
	args: &[&str],
) -> Result<HookResult> {
	let arg_str = format!("{} {}", hook_script, args.join(" "));
	let bash_args = vec!["-c".to_string(), arg_str];

	let output = Command::new("bash")
		.args(bash_args)
		.current_dir(path)
		// This call forces Command to handle the Path environment correctly on windows,
		// the specific env set here does not matter
		// see https://github.com/rust-lang/rust/issues/37519
		.env(
			"DUMMY_ENV_TO_FIX_WINDOWS_CMD_RUNS",
			"FixPathHandlingOnWindows",
		)
		.output()?;

	if output.status.success() {
		Ok(HookResult::Ok)
	} else {
		let err = String::from_utf8_lossy(&output.stderr);
		let out = String::from_utf8_lossy(&output.stdout);
		let formatted = format!("{}{}", out, err);

		Ok(HookResult::NotOk(formatted))
	}
}

#[cfg(not(windows))]
fn is_executable(path: &Path) -> bool {
	use std::os::unix::fs::PermissionsExt;
	let metadata = match path.metadata() {
		Ok(metadata) => metadata,
		Err(_) => return false,
	};

	let permissions = metadata.permissions();
	permissions.mode() & 0o111 != 0
}

#[cfg(windows)]
/// windows does not consider bash scripts to be executable so we consider everything
/// to be executable (which is not far from the truth for windows platform.)
const fn is_executable(_: &Path) -> bool {
	true
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::sync::tests::repo_init;
	use std::fs::{self, File};

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

		let hook = b"#!/bin/sh
exit 0
        ";

		create_hook(root, HOOK_COMMIT_MSG, hook);

		let mut msg = String::from("test");
		let res = hooks_commit_msg(repo_path, &mut msg).unwrap();

		assert_eq!(res, HookResult::Ok);

		assert_eq!(msg, String::from("test"));
	}

	#[test]
	fn test_pre_commit_sh() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path = root.as_os_str().to_str().unwrap();

		let hook = b"#!/bin/sh
exit 0
        ";

		create_hook(root, HOOK_PRE_COMMIT, hook);
		let res = hooks_pre_commit(repo_path).unwrap();
		assert_eq!(res, HookResult::Ok);
	}

	#[test]
	fn test_pre_commit_fail_sh() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path = root.as_os_str().to_str().unwrap();

		let hook = b"#!/bin/sh
echo 'rejected'        
exit 1
        ";

		create_hook(root, HOOK_PRE_COMMIT, hook);
		let res = hooks_pre_commit(repo_path).unwrap();
		assert!(res != HookResult::Ok);
	}

	#[test]
	fn test_pre_commit_py() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path = root.as_os_str().to_str().unwrap();

		// mirror how python pre-commmit sets itself up
		#[cfg(not(windows))]
		let hook = b"#!/usr/bin/env python
import sys
sys.exit(0)
        ";
		#[cfg(windows)]
		let hook = b"#!/bin/env python.exe
import sys
sys.exit(0)
        ";

		create_hook(root, HOOK_PRE_COMMIT, hook);
		let res = hooks_pre_commit(repo_path).unwrap();
		assert_eq!(res, HookResult::Ok);
	}

	#[test]
	fn test_pre_commit_fail_py() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path = root.as_os_str().to_str().unwrap();

		// mirror how python pre-commmit sets itself up
		#[cfg(not(windows))]
		let hook = b"#!/usr/bin/env python
import sys
sys.exit(1)
        ";
		#[cfg(windows)]
		let hook = b"#!/bin/env python.exe
import sys
sys.exit(1)
        ";

		create_hook(root, HOOK_PRE_COMMIT, hook);
		let res = hooks_pre_commit(repo_path).unwrap();
		assert!(res != HookResult::Ok);
	}

	#[test]
	fn test_hooks_commit_msg_reject() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path = root.as_os_str().to_str().unwrap();

		let hook = b"#!/bin/sh
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
	fn test_hooks_commit_msg_reject_in_subfolder() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		// let repo_path = root.as_os_str().to_str().unwrap();

		let hook = b"#!/bin/sh
echo 'msg' > $1
echo 'rejected'
exit 1
        ";

		create_hook(root, HOOK_COMMIT_MSG, hook);

		let subfolder = root.join("foo/");
		fs::create_dir_all(&subfolder).unwrap();

		let mut msg = String::from("test");
		let res =
			hooks_commit_msg(subfolder.to_str().unwrap(), &mut msg)
				.unwrap();

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

		let hook = b"#!/bin/sh
echo 'msg' > $1
exit 0
        ";

		create_hook(root, HOOK_COMMIT_MSG, hook);

		let mut msg = String::from("test");
		let res = hooks_commit_msg(repo_path, &mut msg).unwrap();

		assert_eq!(res, HookResult::Ok);
		assert_eq!(msg, String::from("msg\n"));
	}

	#[test]
	fn test_post_commit_hook_reject_in_subfolder() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();

		let hook = b"#!/bin/sh
echo 'rejected'
exit 1
        ";

		create_hook(root, HOOK_POST_COMMIT, hook);

		let subfolder = root.join("foo/");
		fs::create_dir_all(&subfolder).unwrap();

		let res =
			hooks_post_commit(subfolder.to_str().unwrap()).unwrap();

		assert_eq!(
			res,
			HookResult::NotOk(String::from("rejected\n"))
		);
	}
}
