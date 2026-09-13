use std::fmt;

use envfish_vault::{ExposeSecret, SecretString};

/// A plaintext secret value while it is in memory.
///
/// Guarantees:
/// - `Debug` prints `SecretValue([REDACTED])`.
/// - No `Display`, `Serialize`, `Deserialize`, `Clone` or `PartialEq` — a secret
///   cannot end up in a log line, JSON payload, or Tauri IPC response by accident.
/// - The inner buffer is zeroized on drop (via `secrecy`).
///
/// The only way to read the content is [`SecretValue::expose`], whose name is easy
/// to grep for in review.
pub struct SecretValue(SecretString);

impl SecretValue {
    pub fn new(value: impl Into<String>) -> Self {
        Self(SecretString::from(value.into()))
    }

    /// Borrow the plaintext. Keep the borrow as short as possible.
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }

    pub(crate) fn as_secret_string(&self) -> &SecretString {
        &self.0
    }
}

impl From<SecretString> for SecretValue {
    fn from(value: SecretString) -> Self {
        Self(value)
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretValue([REDACTED])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_is_redacted() {
        let s = SecretValue::new("sk-abc");
        let out = format!("{s:?}");
        assert_eq!(out, "SecretValue([REDACTED])");
        assert!(!out.contains("sk-abc"));
    }
}
