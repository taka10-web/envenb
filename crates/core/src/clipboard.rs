//! Human-only clipboard hand-off with automatic clearing.
//!
//! The value is copied and, after `ttl`, the clipboard is cleared again if it
//! still holds that value (so a later copy by the user is not destroyed). The
//! value never appears in logs or return values.

use std::time::Duration;

pub const DEFAULT_TTL: Duration = Duration::from_secs(30);

/// Copy `value` to the OS clipboard. Returns immediately; a background thread
/// clears the clipboard after `ttl`. Callers must only invoke this for human
/// initiated actions (TTY / desktop click), never from an AI-facing path.
pub fn copy_then_clear(value: &str, ttl: Duration) -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_text(value.to_string()).map_err(|e| e.to_string())?;
    let expected = value.to_string();
    std::thread::spawn(move || {
        std::thread::sleep(ttl);
        if let Ok(mut cb) = arboard::Clipboard::new()
            && cb.get_text().map(|t| t == expected).unwrap_or(false)
        {
            let _ = cb.clear();
        }
    });
    Ok(())
}
