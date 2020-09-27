use anyhow::Result;
#[cfg(feature = "clipboard")]
use clipboard::{ClipboardContext, ClipboardProvider};

#[cfg(feature = "clipboard")]
pub fn copy_string(string: String) -> Result<()> {
    use anyhow::anyhow;

    let mut ctx: ClipboardContext = ClipboardProvider::new()
        .map_err(|e| {
            anyhow!("failed to get access to clipboard: {}", e)
        })?;
    ctx.set_contents(string).map_err(|e| {
        anyhow!("failed to set clipboard contents: {}", e)
    })?;

    Ok(())
}

#[cfg(not(feature = "clipboard"))]
pub fn copy_string(_string: String) -> Result<()> {
    Ok(())
}

#[cfg(feature = "clipboard")]
pub const fn is_supported() -> bool {
    true
}

#[cfg(not(feature = "clipboard"))]
pub fn is_supported() -> bool {
    false
}
