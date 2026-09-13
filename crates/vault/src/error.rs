use std::path::PathBuf;

/// Errors produced by the vault. Variants never carry plaintext or key material.
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("master key file has an unexpected length ({found} bytes, expected {expected})")]
    InvalidKeyLength { found: usize, expected: usize },

    #[error("failed to read master key at {path}: {source}")]
    KeyRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write master key at {path}: {source}")]
    KeyWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("nonce has an unexpected length ({found} bytes, expected {expected})")]
    InvalidNonceLength { found: usize, expected: usize },

    /// Wrong key, tampered ciphertext, or mismatched associated data.
    /// Deliberately opaque: the AEAD does not tell us which, and neither should we.
    #[error("secret could not be decrypted (wrong key, corrupted data, or context mismatch)")]
    Decrypt,

    #[error("encryption failed")]
    Encrypt,

    #[error("decrypted secret is not valid UTF-8")]
    InvalidUtf8,

    #[error("os keychain error: {0}")]
    Keychain(String),
}
