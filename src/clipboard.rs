use anyhow::{anyhow, Result};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use which::which;

fn exec_copy_with_args(
	command: &str,
	args: &[&str],
	text: &str,
) -> Result<()> {
	let binary = which(command)
		.ok()
		.unwrap_or_else(|| PathBuf::from(command));

	let mut process = Command::new(binary)
		.args(args)
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

fn exec_copy(command: &str, text: &str) -> Result<()> {
	exec_copy_with_args(command, &[], text)
}

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
pub fn copy_string(text: &str) -> Result<()> {
	if std::env::var("WAYLAND_DISPLAY").is_ok() {
		return exec_copy("wl-copy", text);
	}

	if exec_copy_with_args(
		"xclip",
		&["-selection", "clipboard"],
		text,
	)
	.is_err()
	{
		return exec_copy_with_args("xsel", &["--clipboard"], text);
	}

	Ok(())
}

#[cfg(target_os = "macos")]
pub fn copy_string(text: &str) -> Result<()> {
	exec_copy("pbcopy", text)
}

#[cfg(windows)]
pub fn copy_string(text: &str) -> Result<()> {
	exec_copy("clip", text)
}
