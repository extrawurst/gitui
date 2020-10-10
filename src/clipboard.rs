use anyhow::Result;
use std::io::Write;
use std::process::{Command, Stdio};

fn execute_copy_command(
    command: &mut Command,
    string: &str,
) -> Result<()> {
    use anyhow::anyhow;

    let mut process = command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .map_err(|e| anyhow!("`{:?}`: {}", command, e))?;

    process
        .stdin
        .as_mut()
        .ok_or_else(|| anyhow!("`{:?}`", command))?
        .write_all(string.as_bytes())
        .map_err(|e| anyhow!("`{:?}`: {}", command, e))?;

    process
        .wait()
        .map_err(|e| anyhow!("`{:?}`: {}", command, e))?;

    Ok(())
}

#[cfg(target_os = "linux")]
pub fn copy_string(string: &str) -> Result<()> {
    execute_copy_command(
        Command::new("xclip").arg("-selection").arg("clipboard"),
        string,
    )
}

#[cfg(target_os = "macos")]
pub fn copy_string(string: &str) -> Result<()> {
    execute_copy_command(&mut Command::new("pbcopy"), string)
}

#[cfg(windows)]
pub fn copy_string(string: &str) -> Result<()> {
    execute_copy_command(&mut Command::new("clip"), string)
}
