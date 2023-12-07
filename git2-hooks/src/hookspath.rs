use git2::Repository;

use crate::{error::Result, HookResult, HooksError};

use std::{
	path::Path, path::PathBuf, process::Command, str::FromStr,
};

pub struct HookPaths {
	pub git: PathBuf,
	pub hook: PathBuf,
	pub pwd: PathBuf,
}

impl HookPaths {
	pub fn new(repo: &Repository, hook: &str) -> Result<Self> {
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
				.ok_or(HooksError::PathToString)?,
		)?;

		let hook = PathBuf::from_str(hook.as_ref())
			.map_err(|_| HooksError::PathToString)?;

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
			let stderr =
				String::from_utf8_lossy(&output.stderr).to_string();
			let stdout =
				String::from_utf8_lossy(&output.stdout).to_string();

			Ok(HookResult::NotOk { stdout, stderr })
		}
	}
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
