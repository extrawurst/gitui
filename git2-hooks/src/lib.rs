//! git2-rs addon supporting git hooks
//!
//! most basic hook is: [`hooks_pre_commit`]. see also other `hooks_*` functions
//!
//! [`create_hook`] is useful to create git hooks from code (unittest make heavy usage of it)
mod error;
mod hookspath;

use std::{
	fs::File,
	io::{Read, Write},
	path::{Path, PathBuf},
	process::Command,
};

pub use error::HooksError;
use error::Result;
use hookspath::HookPaths;

use git2::Repository;

pub const HOOK_POST_COMMIT: &str = "post-commit";
pub const HOOK_PRE_COMMIT: &str = "pre-commit";
pub const HOOK_COMMIT_MSG: &str = "commit-msg";

const HOOK_COMMIT_MSG_TEMP_FILE: &str = "COMMIT_EDITMSG";

///
#[derive(Debug, PartialEq, Eq)]
pub enum HookResult {
	/// Everything went fine
	Ok,
	/// Hook returned error
	NotOk { stdout: String, stderr: String },
}

/// helper method to create git hooks programmatically (heavy used in unittests)
pub fn create_hook(
	r: &Repository,
	hook: &str,
	hook_script: &[u8],
) -> PathBuf {
	let hook = HookPaths::new(r, hook).unwrap();

	let path = hook.hook.clone();

	create_hook_in_path(&hook.hook, hook_script);

	path
}

fn create_hook_in_path(path: &Path, hook_script: &[u8]) {
	File::create(path).unwrap().write_all(hook_script).unwrap();

	#[cfg(not(windows))]
	{
		Command::new("chmod")
			.arg("+x")
			.arg(path)
			// .current_dir(path)
			.output()
			.unwrap();
	}
}

/// this hook is documented here <https://git-scm.com/docs/githooks#_commit_msg>
/// we use the same convention as other git clients to create a temp file containing
/// the commit message at `<.git|hooksPath>/COMMIT_EDITMSG` and pass it's relative path as the only
/// parameter to the hook script.
pub fn hooks_commit_msg(
	repo: &Repository,
	msg: &mut String,
) -> Result<HookResult> {
	let hooks_path = HookPaths::new(repo, HOOK_COMMIT_MSG)?;

	if hooks_path.is_executable() {
		let temp_file =
			hooks_path.git.join(HOOK_COMMIT_MSG_TEMP_FILE);
		File::create(&temp_file)?.write_all(msg.as_bytes())?;

		let res = hooks_path.run_hook(&[temp_file
			.as_os_str()
			.to_string_lossy()
			.as_ref()])?;

		// load possibly altered msg
		msg.clear();
		File::open(temp_file)?.read_to_string(msg)?;

		Ok(res)
	} else {
		Ok(HookResult::Ok)
	}
}

/// this hook is documented here <https://git-scm.com/docs/githooks#_pre_commit>
pub fn hooks_pre_commit(repo: &Repository) -> Result<HookResult> {
	let hook = HookPaths::new(repo, HOOK_PRE_COMMIT)?;

	if hook.is_executable() {
		Ok(hook.run_hook(&[])?)
	} else {
		Ok(HookResult::Ok)
	}
}

/// this hook is documented here <https://git-scm.com/docs/githooks#_post_commit>
pub fn hooks_post_commit(repo: &Repository) -> Result<HookResult> {
	let hook = HookPaths::new(repo, HOOK_POST_COMMIT)?;

	if hook.is_executable() {
		Ok(hook.run_hook(&[])?)
	} else {
		Ok(HookResult::Ok)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use git2_testing::{repo_init, repo_init_bare};
	use pretty_assertions::assert_eq;
	use tempfile::TempDir;

	#[test]
	fn test_smoke() {
		let (_td, repo) = repo_init();

		let mut msg = String::from("test");
		let res = hooks_commit_msg(&repo, &mut msg).unwrap();

		assert_eq!(res, HookResult::Ok);

		let res = hooks_post_commit(&repo).unwrap();

		assert_eq!(res, HookResult::Ok);
	}

	#[test]
	fn test_hooks_commit_msg_ok() {
		let (_td, repo) = repo_init();

		let hook = b"#!/bin/sh
exit 0
        ";

		create_hook(&repo, HOOK_COMMIT_MSG, hook);

		let mut msg = String::from("test");
		let res = hooks_commit_msg(&repo, &mut msg).unwrap();

		assert_eq!(res, HookResult::Ok);

		assert_eq!(msg, String::from("test"));
	}

	#[test]
	fn test_hooks_commit_msg_with_shell_command_ok() {
		let (_td, repo) = repo_init();

		let hook = br#"#!/bin/sh
COMMIT_MSG="$(cat "$1")"
printf "$COMMIT_MSG" | sed 's/sth/shell_command/g' >"$1"
exit 0
        "#;

		create_hook(&repo, HOOK_COMMIT_MSG, hook);

		let mut msg = String::from("test_sth");
		let res = hooks_commit_msg(&repo, &mut msg).unwrap();

		assert_eq!(res, HookResult::Ok);

		assert_eq!(msg, String::from("test_shell_command"));
	}

	#[test]
	fn test_pre_commit_sh() {
		let (_td, repo) = repo_init();

		let hook = b"#!/bin/sh
exit 0
        ";

		create_hook(&repo, HOOK_PRE_COMMIT, hook);
		let res = hooks_pre_commit(&repo).unwrap();
		assert_eq!(res, HookResult::Ok);
	}

	#[test]
	fn test_pre_commit_fail_sh() {
		let (_td, repo) = repo_init();

		let hook = b"#!/bin/sh
echo 'rejected'
exit 1
        ";

		create_hook(&repo, HOOK_PRE_COMMIT, hook);
		let res = hooks_pre_commit(&repo).unwrap();
		assert!(res != HookResult::Ok);
	}

	#[test]
	fn test_env_containing_path() {
		let (_td, repo) = repo_init();

		let hook = b"#!/bin/sh
export
exit 1
        ";

		create_hook(&repo, HOOK_PRE_COMMIT, hook);
		let res = hooks_pre_commit(&repo).unwrap();

		let HookResult::NotOk { stdout, .. } = res else {
			unreachable!()
		};

		assert!(stdout
			.lines()
			.any(|line| line.starts_with("export PATH")));
	}

	#[test]
	fn test_pre_commit_fail_hookspath() {
		let (_td, repo) = repo_init();
		let hooks = TempDir::new().unwrap();

		let hook = b"#!/bin/sh
echo 'rejected'
exit 1
        ";

		create_hook_in_path(&hooks.path().join("pre-commit"), hook);

		repo.config()
			.unwrap()
			.set_str(
				"core.hooksPath",
				hooks.path().as_os_str().to_str().unwrap(),
			)
			.unwrap();

		let res = hooks_pre_commit(&repo).unwrap();

		assert_eq!(
			res,
			HookResult::NotOk {
				stdout: String::from("rejected\n"),
				stderr: String::new()
			}
		);
	}

	#[test]
	fn test_pre_commit_fail_bare() {
		let (_td, repo) = repo_init_bare();

		let hook = b"#!/bin/sh
echo 'rejected'
exit 1
        ";

		create_hook(&repo, HOOK_PRE_COMMIT, hook);
		let res = hooks_pre_commit(&repo).unwrap();
		assert!(res != HookResult::Ok);
	}

	#[test]
	fn test_pre_commit_py() {
		let (_td, repo) = repo_init();

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

		create_hook(&repo, HOOK_PRE_COMMIT, hook);
		let res = hooks_pre_commit(&repo).unwrap();
		assert_eq!(res, HookResult::Ok);
	}

	#[test]
	fn test_pre_commit_fail_py() {
		let (_td, repo) = repo_init();

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

		create_hook(&repo, HOOK_PRE_COMMIT, hook);
		let res = hooks_pre_commit(&repo).unwrap();
		assert!(res != HookResult::Ok);
	}

	#[test]
	fn test_hooks_commit_msg_reject() {
		let (_td, repo) = repo_init();

		let hook = b"#!/bin/sh
echo 'msg' > $1
echo 'rejected'
exit 1
        ";

		create_hook(&repo, HOOK_COMMIT_MSG, hook);

		let mut msg = String::from("test");
		let res = hooks_commit_msg(&repo, &mut msg).unwrap();

		assert_eq!(
			res,
			HookResult::NotOk {
				stdout: String::from("rejected\n"),
				stderr: String::new()
			}
		);

		assert_eq!(msg, String::from("msg\n"));
	}

	#[test]
	fn test_commit_msg_no_block_but_alter() {
		let (_td, repo) = repo_init();

		let hook = b"#!/bin/sh
echo 'msg' > $1
exit 0
        ";

		create_hook(&repo, HOOK_COMMIT_MSG, hook);

		let mut msg = String::from("test");
		let res = hooks_commit_msg(&repo, &mut msg).unwrap();

		assert_eq!(res, HookResult::Ok);
		assert_eq!(msg, String::from("msg\n"));
	}

	#[test]
	fn test_hook_pwd_in_bare_without_workdir() {
		let (_td, repo) = repo_init_bare();
		let git_root = repo.path().to_path_buf();

		let hook = HookPaths::new(&repo, HOOK_POST_COMMIT).unwrap();

		assert_eq!(hook.pwd, git_root);
	}

	#[test]
	fn test_hook_pwd() {
		let (_td, repo) = repo_init();
		let git_root = repo.path().to_path_buf();

		let hook = HookPaths::new(&repo, HOOK_POST_COMMIT).unwrap();

		assert_eq!(hook.pwd, git_root.parent().unwrap());
	}
}
