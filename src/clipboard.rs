use anyhow::{anyhow, Result};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use which::which;

fn exec_copy_with_args(
	command: &str,
	args: &[&str],
	text: &str,
	pipe_stderr: bool,
) -> Result<()> {
	let binary = which(command)
		.ok()
		.unwrap_or_else(|| PathBuf::from(command));

	let mut process = Command::new(binary)
		.args(args)
		.stdin(Stdio::piped())
		.stdout(Stdio::null())
		.stderr(if pipe_stderr {
			Stdio::piped()
		} else {
			Stdio::null()
		})
		.spawn()
		.map_err(|e| anyhow!("`{:?}`: {}", command, e))?;

	process
		.stdin
		.as_mut()
		.ok_or_else(|| anyhow!("`{:?}`", command))?
		.write_all(text.as_bytes())
		.map_err(|e| anyhow!("`{:?}`: {}", command, e))?;

	let out = process
		.wait_with_output()
		.map_err(|e| anyhow!("`{:?}`: {}", command, e))?;

	if out.status.success() {
		Ok(())
	} else {
		let msg = if out.stderr.is_empty() {
			format!("{}", out.status).into()
		} else {
			String::from_utf8_lossy(&out.stderr)
		};
		Err(anyhow!("`{command:?}`: {msg}"))
	}
}

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
pub fn copy_string(text: &str) -> Result<()> {
	if std::env::var("WAYLAND_DISPLAY").is_ok() {
		return exec_copy_with_args("wl-copy", &[], text, false);
	}

	if exec_copy_with_args(
		"xclip",
		&["-selection", "clipboard"],
		text,
		false,
	)
	.is_err()
	{
		return exec_copy_with_args(
			"xsel",
			&["--clipboard"],
			text,
			true,
		);
	}

	Ok(())
}

#[cfg(any(target_os = "macos", windows))]
fn exec_copy(command: &str, text: &str) -> Result<()> {
	exec_copy_with_args(command, &[], text, true)
}

#[cfg(target_os = "macos")]
pub fn copy_string(text: &str) -> Result<()> {
	exec_copy("pbcopy", text)
}

#[cfg(windows)]
pub fn copy_string(text: &str) -> Result<()> {
	exec_copy("clip", text)
}
