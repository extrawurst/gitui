use anyhow::{anyhow, Result};
#[cfg(target_family = "unix")]
#[cfg(not(target_os = "macos"))]
use std::ffi::OsStr;
use std::io::Write;
use std::process::{Command, Stdio};

fn execute_copy_command(command: Command, text: &str) -> Result<()> {
	let mut command = command;

	let mut process = command
		.stdin(Stdio::piped())
		.stdout(Stdio::null())
		.spawn()
		.map_err(|e| anyhow!("`{:?}`: {}", command, e))?;

	process
		.stdin
		.as_mut()
		.ok_or_else(|| anyhow!("`{:?}`", command))?
		.write_all(text.as_bytes())
		.map_err(|e| anyhow!("`{:?}`: {}", command, e))?;

	process
		.wait()
		.map_err(|e| anyhow!("`{:?}`: {}", command, e))?;

	Ok(())
}

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
fn gen_command(
	path: impl AsRef<OsStr>,
	xclip_syntax: bool,
) -> Command {
	let mut c = Command::new(path);
	if xclip_syntax {
		c.arg("-selection");
		c.arg("clipboard");
	} else {
		c.arg("--clipboard");
	}
	c
}

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
pub fn copy_string(string: &str) -> Result<()> {
	use std::path::PathBuf;
	use which::which;
	let (path, xclip_syntax) = which("xclip").ok().map_or_else(
		|| {
			(
				which("xsel")
					.ok()
					.unwrap_or_else(|| PathBuf::from("xsel")),
				false,
			)
		},
		|path| (path, true),
	);

	let cmd = gen_command(path, xclip_syntax);
	execute_copy_command(cmd, string)
}

#[cfg(target_os = "macos")]
pub fn copy_string(string: &str) -> Result<()> {
	execute_copy_command(Command::new("pbcopy"), string)
}

#[cfg(windows)]
pub fn copy_string(string: &str) -> Result<()> {
	execute_copy_command(Command::new("clip"), string)
}
