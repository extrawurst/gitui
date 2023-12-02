use super::{repository::repo, RepoPath};
use crate::error::{self, Result};
use scopetime::scope_time;
use std::{
	fs::File,
	io::{Read, Write},
	path::{Path, PathBuf},
	process::Command,
	str::FromStr,
};

const HOOK_POST_COMMIT: &str = "post-commit";
const HOOK_PRE_COMMIT: &str = "pre-commit";
const HOOK_COMMIT_MSG: &str = "commit-msg";
const HOOK_COMMIT_MSG_TEMP_FILE: &str = "COMMIT_EDITMSG";

struct HookPaths {
	git: PathBuf,
	hook: PathBuf,
	pwd: PathBuf,
}

impl HookPaths {
	pub fn new(repo_path: &RepoPath, hook: &str) -> Result<Self> {
		let repo = repo(repo_path)?;
		let pwd = repo
			.workdir()
			.unwrap_or_else(|| repo.path())
			.to_path_buf();

		let git_dir = repo.path().to_path_buf();
		let hooks_path = repo
			.config()
			.and_then(|config| config.get_string("core.hooksPath"))
			.map_or_else(
				|e| {
					log::error!("hookspath error: {}", e);
					repo.path().to_path_buf().join("hooks/")
				},
				PathBuf::from,
			);

		let hook = hooks_path.join(hook);

		let hook = shellexpand::full(
			hook.as_os_str()
				.to_str()
				.ok_or(error::Error::PathString)?,
		)?;

		let hook = PathBuf::from_str(hook.as_ref())
			.map_err(|_| error::Error::PathString)?;

		Ok(Self {
			git: git_dir,
			hook,
			pwd,
		})
	}

	pub fn is_executable(&self) -> bool {
		self.hook.exists() && is_executable(&self.hook)
	}

	/// this function calls hook scripts based on conventions documented here
	/// see <https://git-scm.com/docs/githooks>
	pub fn run_hook(&self, args: &[&str]) -> Result<HookResult> {
		let arg_str = format!("{:?} {}", self.hook, args.join(" "));
		// Use -l to avoid "command not found" on Windows.
		let bash_args =
			vec!["-l".to_string(), "-c".to_string(), arg_str];

		log::trace!("run hook '{:?}' in '{:?}'", self.hook, self.pwd);

		let git_bash = find_bash_executable()
			.unwrap_or_else(|| PathBuf::from("bash"));
		let output = Command::new(git_bash)
			.args(bash_args)
			.current_dir(&self.pwd)
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
			let formatted = format!("{out}{err}");

			Ok(HookResult::NotOk(formatted))
		}
	}
}

/// this hook is documented here <https://git-scm.com/docs/githooks#_commit_msg>
/// we use the same convention as other git clients to create a temp file containing
/// the commit message at `<.git|hooksPath>/COMMIT_EDITMSG` and pass it's relative path as the only
/// parameter to the hook script.
pub fn hooks_commit_msg(
	repo_path: &RepoPath,
	msg: &mut String,
) -> Result<HookResult> {
	scope_time!("hooks_commit_msg");

	let hooks_path = HookPaths::new(repo_path, HOOK_COMMIT_MSG)?;

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
///
pub fn hooks_pre_commit(repo_path: &RepoPath) -> Result<HookResult> {
	scope_time!("hooks_pre_commit");

	let hook = HookPaths::new(repo_path, HOOK_PRE_COMMIT)?;

	if hook.is_executable() {
		Ok(hook.run_hook(&[])?)
	} else {
		Ok(HookResult::Ok)
	}
}
///
pub fn hooks_post_commit(repo_path: &RepoPath) -> Result<HookResult> {
	scope_time!("hooks_post_commit");

	let hook = HookPaths::new(repo_path, HOOK_POST_COMMIT)?;

	if hook.is_executable() {
		Ok(hook.run_hook(&[])?)
	} else {
		Ok(HookResult::Ok)
	}
}

///
#[derive(Debug, PartialEq, Eq)]
pub enum HookResult {
	/// Everything went fine
	Ok,
	/// Hook returned error
	NotOk(String),
}

#[cfg(not(windows))]
fn is_executable(path: &Path) -> bool {
	use std::os::unix::fs::PermissionsExt;
	let metadata = match path.metadata() {
		Ok(metadata) => metadata,
		Err(e) => {
			log::error!("metadata error: {}", e);
			return false;
		}
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

// Find bash.exe, and avoid finding wsl's bash.exe on Windows.
// None for non-Windows.
fn find_bash_executable() -> Option<PathBuf> {
	if cfg!(windows) {
		Command::new("where.exe")
			.arg("git")
			.output()
			.ok()
			.map(|out| {
				PathBuf::from(Into::<String>::into(
					String::from_utf8_lossy(&out.stdout),
				))
			})
			.as_deref()
			.and_then(Path::parent)
			.and_then(Path::parent)
			.map(|p| p.join("usr/bin/bash.exe"))
			.filter(|p| p.exists())
	} else {
		None
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::sync::tests::{repo_init, repo_init_bare};
	use std::fs::{self, File};
	use tempfile::TempDir;

	#[test]
	fn test_smoke() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		let mut msg = String::from("test");
		let res = hooks_commit_msg(repo_path, &mut msg).unwrap();

		assert_eq!(res, HookResult::Ok);

		let res = hooks_post_commit(repo_path).unwrap();

		assert_eq!(res, HookResult::Ok);
	}

	fn create_hook(
		path: &RepoPath,
		hook: &str,
		hook_script: &[u8],
	) -> PathBuf {
		let hook = HookPaths::new(path, hook).unwrap();

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

	#[test]
	fn test_hooks_commit_msg_ok() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		let hook = b"#!/bin/sh
exit 0
        ";

		create_hook(repo_path, HOOK_COMMIT_MSG, hook);

		let mut msg = String::from("test");
		let res = hooks_commit_msg(repo_path, &mut msg).unwrap();

		assert_eq!(res, HookResult::Ok);

		assert_eq!(msg, String::from("test"));
	}

	#[test]
	fn test_hooks_commit_msg_with_shell_command_ok() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		let hook = br#"#!/bin/sh
COMMIT_MSG="$(cat "$1")"
printf "$COMMIT_MSG" | sed 's/sth/shell_command/g' >"$1"
exit 0
        "#;

		create_hook(repo_path, HOOK_COMMIT_MSG, hook);

		let mut msg = String::from("test_sth");
		let res = hooks_commit_msg(repo_path, &mut msg).unwrap();

		assert_eq!(res, HookResult::Ok);

		assert_eq!(msg, String::from("test_shell_command"));
	}

	#[test]
	fn test_pre_commit_sh() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		let hook = b"#!/bin/sh
exit 0
        ";

		create_hook(repo_path, HOOK_PRE_COMMIT, hook);
		let res = hooks_pre_commit(repo_path).unwrap();
		assert_eq!(res, HookResult::Ok);
	}

	#[test]
	fn test_pre_commit_fail_sh() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		let hook = b"#!/bin/sh
echo 'rejected'        
exit 1
        ";

		create_hook(repo_path, HOOK_PRE_COMMIT, hook);
		let res = hooks_pre_commit(repo_path).unwrap();
		assert!(res != HookResult::Ok);
	}

	#[test]
	fn test_env_containing_path() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		let hook = b"#!/bin/sh
export
exit 1
        ";

		create_hook(repo_path, HOOK_PRE_COMMIT, hook);
		let res = hooks_pre_commit(repo_path).unwrap();

		let HookResult::NotOk(out) = res else {
			unreachable!()
		};

		assert!(out
			.lines()
			.any(|line| line.starts_with("export PATH")));
	}

	#[test]
	fn test_pre_commit_fail_hookspath() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let hooks = TempDir::new().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

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
		let res = hooks_pre_commit(repo_path).unwrap();
		assert_eq!(
			res,
			HookResult::NotOk(String::from("rejected\n"))
		);
	}

	#[test]
	fn test_pre_commit_fail_bare() {
		let (git_root, _repo) = repo_init_bare().unwrap();
		let workdir = TempDir::new().unwrap();
		let git_root = git_root.into_path();
		let repo_path = &RepoPath::Workdir {
			gitdir: dbg!(git_root),
			workdir: dbg!(workdir.into_path()),
		};

		let hook = b"#!/bin/sh
echo 'rejected'        
exit 1
        ";

		create_hook(repo_path, HOOK_PRE_COMMIT, hook);
		let res = hooks_pre_commit(repo_path).unwrap();
		assert!(res != HookResult::Ok);
	}

	#[test]
	fn test_pre_commit_py() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

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

		create_hook(repo_path, HOOK_PRE_COMMIT, hook);
		let res = hooks_pre_commit(repo_path).unwrap();
		assert_eq!(res, HookResult::Ok);
	}

	#[test]
	fn test_pre_commit_fail_py() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

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

		create_hook(repo_path, HOOK_PRE_COMMIT, hook);
		let res = hooks_pre_commit(repo_path).unwrap();
		assert!(res != HookResult::Ok);
	}

	#[test]
	fn test_hooks_commit_msg_reject() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		let hook = b"#!/bin/sh
echo 'msg' > $1
echo 'rejected'
exit 1
        ";

		create_hook(repo_path, HOOK_COMMIT_MSG, hook);

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
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		let hook = b"#!/bin/sh
echo 'msg' > $1
echo 'rejected'
exit 1
        ";

		create_hook(repo_path, HOOK_COMMIT_MSG, hook);

		let subfolder = root.join("foo/");
		fs::create_dir_all(&subfolder).unwrap();

		let mut msg = String::from("test");
		let res = hooks_commit_msg(
			&subfolder.to_str().unwrap().into(),
			&mut msg,
		)
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
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		let hook = b"#!/bin/sh
echo 'msg' > $1
exit 0
        ";

		create_hook(repo_path, HOOK_COMMIT_MSG, hook);

		let mut msg = String::from("test");
		let res = hooks_commit_msg(repo_path, &mut msg).unwrap();

		assert_eq!(res, HookResult::Ok);
		assert_eq!(msg, String::from("msg\n"));
	}

	#[test]
	fn test_post_commit_hook_reject_in_subfolder() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		let hook = b"#!/bin/sh
echo 'rejected'
exit 1
        ";

		create_hook(repo_path, HOOK_POST_COMMIT, hook);

		let subfolder = root.join("foo/");
		fs::create_dir_all(&subfolder).unwrap();

		let res =
			hooks_post_commit(&subfolder.to_str().unwrap().into())
				.unwrap();

		assert_eq!(
			res,
			HookResult::NotOk(String::from("rejected\n"))
		);
	}

	// make sure we run the hooks with the correct pwd.
	// for non-bare repos this is the dir of the worktree
	// unfortunately does not work on windows
	#[test]
	#[cfg(unix)]
	fn test_pre_commit_workdir() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();
		let workdir =
			crate::sync::utils::repo_work_dir(repo_path).unwrap();

		let hook = b"#!/bin/sh
echo $(pwd)
exit 1
        ";

		create_hook(repo_path, HOOK_PRE_COMMIT, hook);
		let res = hooks_pre_commit(repo_path).unwrap();
		if let HookResult::NotOk(res) = res {
			assert_eq!(
				Path::new(res.trim_end()),
				Path::new(&workdir)
			);
		} else {
			assert!(false);
		}
	}

	#[test]
	fn test_hook_pwd_in_bare_without_workdir() {
		let (_td, _repo) = repo_init_bare().unwrap();
		let git_root = _repo.path().to_path_buf();
		let repo_path = &RepoPath::Path(git_root.clone());

		let hook =
			HookPaths::new(repo_path, HOOK_POST_COMMIT).unwrap();

		assert_eq!(hook.pwd, dbg!(git_root));
	}

	#[test]
	fn test_hook_pwd_in_bare_with_workdir() {
		let (git_root, _repo) = repo_init_bare().unwrap();
		let workdir = TempDir::new().unwrap();
		let git_root = git_root.into_path();
		let repo_path = &RepoPath::Workdir {
			gitdir: dbg!(git_root),
			workdir: dbg!(workdir.path().to_path_buf()),
		};

		let hook =
			HookPaths::new(repo_path, HOOK_POST_COMMIT).unwrap();

		assert_eq!(
			hook.pwd.canonicalize().unwrap(),
			dbg!(workdir.path().canonicalize().unwrap())
		);
	}

	#[test]
	fn test_hook_pwd() {
		let (_td, _repo) = repo_init().unwrap();
		let git_root = _repo.path().to_path_buf();
		let repo_path = &RepoPath::Path(git_root.clone());

		let hook =
			HookPaths::new(repo_path, HOOK_POST_COMMIT).unwrap();

		assert_eq!(hook.pwd, git_root.parent().unwrap());
	}
}
