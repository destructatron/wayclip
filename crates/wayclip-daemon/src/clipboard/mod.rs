//! Clipboard monitoring and operations.

mod monitor;

pub use monitor::*;

use anyhow::{anyhow, Result};
use std::io::Write;
use std::process::{Command, Stdio};

/// Event emitted when clipboard content changes.
#[derive(Debug, Clone)]
pub struct ClipboardEvent {
    /// The clipboard content.
    pub content: Vec<u8>,
    /// MIME type of the content.
    pub mime_type: String,
    /// Source application (if available).
    #[allow(dead_code)]
    pub source_app: Option<String>,
}

/// Copy data to the clipboard using wl-copy.
///
/// This spawns wl-copy as a subprocess which handles keeping
/// the clipboard content alive properly.
pub fn copy_to_clipboard(data: &[u8], mime_type: &str) -> Result<()> {
    let mut child = Command::new("wl-copy")
        .arg("--type")
        .arg(mime_type)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("Failed to spawn wl-copy: {}. Is wl-clipboard installed?", e))?;

    // Write data to wl-copy's stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(data)?;
        // stdin is dropped here, closing the pipe
    }

    // Wait for wl-copy to finish initial setup (it forks to background)
    let status = child.wait()?;

    if !status.success() {
        return Err(anyhow!("wl-copy failed with status: {}", status));
    }

    Ok(())
}
