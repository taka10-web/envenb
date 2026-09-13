//! Master key material and the providers that store it.

use std::fmt;
use std::path::{Path, PathBuf};

use rand::RngCore;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::VaultError;

/// Size of the XChaCha20-Poly1305 key in bytes.
pub const MASTER_KEY_LEN: usize = 32;

/// A 256-bit symmetric master key.
///
/// - `Debug` prints a fixed redaction marker.
/// - The bytes are zeroized when the value is dropped.
/// - There is intentionally no `Clone`, `Serialize`, or `Display`.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct MasterKey([u8; MASTER_KEY_LEN]);

impl MasterKey {
    /// Generate a fresh random key using the OS CSPRNG.
    pub fn generate() -> Self {
        let mut bytes = [0u8; MASTER_KEY_LEN];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        Self(bytes)
    }

    /// Build a key from raw bytes. The input slice is **not** zeroized by this function;
    /// callers that own the buffer should zeroize it themselves.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, VaultError> {
        if bytes.len() != MASTER_KEY_LEN {
            return Err(VaultError::InvalidKeyLength {
                found: bytes.len(),
                expected: MASTER_KEY_LEN,
            });
        }
        let mut arr = [0u8; MASTER_KEY_LEN];
        arr.copy_from_slice(bytes);
        Ok(Self(arr))
    }

    /// Borrow the raw key bytes. Only the cipher module and key-storage backends need this.
    pub(crate) fn as_bytes(&self) -> &[u8; MASTER_KEY_LEN] {
        &self.0
    }
}

impl fmt::Debug for MasterKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MasterKey([REDACTED])")
    }
}

/// Abstraction over master-key storage backends.
///
/// Phase 1 ships [`FileMasterKeyProvider`]. Later phases add providers backed by
/// macOS Keychain, Windows Credential Manager / DPAPI and Linux Secret Service.
/// The [`Vault`](crate::Vault) never cares which one is in use.
pub trait MasterKeyProvider: Send + Sync {
    /// Return the existing key, creating and persisting a new one if none exists yet.
    fn load_or_create(&self) -> Result<MasterKey, VaultError>;

    /// Human-readable description of where the key is kept (for `envfish status`).
    /// Must not reveal key material.
    fn describe(&self) -> String;
}

/// Stores the master key as a raw 32-byte file with `0600` permissions.
///
/// This is the Phase 1 backend. It keeps the key out of SQLite so that copying the
/// database alone is not enough to recover secrets, but it is *not* as strong as an
/// OS keychain: any process running as the same user can read the file.
pub struct FileMasterKeyProvider {
    path: PathBuf,
}

impl FileMasterKeyProvider {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Write an existing key (used when migrating from another backend). Fails if a
    /// key file already exists, so a live key is never overwritten by accident.
    pub fn store(&self, key: &MasterKey) -> Result<(), VaultError> {
        self.write_bytes(key.as_bytes())
    }

    pub fn delete(&self) -> Result<(), VaultError> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(VaultError::KeyWrite {
                path: self.path.clone(),
                source,
            }),
        }
    }

    fn write_new_key(&self) -> Result<MasterKey, VaultError> {
        let key = MasterKey::generate();
        self.write_bytes(key.as_bytes())?;
        Ok(key)
    }

    fn write_bytes(&self, bytes: &[u8; MASTER_KEY_LEN]) -> Result<(), VaultError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| VaultError::KeyWrite {
                path: self.path.clone(),
                source,
            })?;
        }

        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }

        let mut file = options.open(&self.path).map_err(|source| VaultError::KeyWrite {
            path: self.path.clone(),
            source,
        })?;
        std::io::Write::write_all(&mut file, bytes).map_err(|source| VaultError::KeyWrite {
            path: self.path.clone(),
            source,
        })?;
        tracing::info!(path = %self.path.display(), "wrote master key file");
        Ok(())
    }
}

impl MasterKeyProvider for FileMasterKeyProvider {
    fn load_or_create(&self) -> Result<MasterKey, VaultError> {
        match std::fs::read(&self.path) {
            Ok(mut bytes) => {
                let key = MasterKey::from_bytes(&bytes);
                bytes.zeroize();
                key
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => self.write_new_key(),
            Err(source) => Err(VaultError::KeyRead {
                path: self.path.clone(),
                source,
            }),
        }
    }

    fn describe(&self) -> String {
        format!("file ({})", self.path.display())
    }
}

/// Keeps a key in memory only. Intended for tests and throwaway sessions.
pub struct InMemoryMasterKeyProvider {
    key_bytes: std::sync::Mutex<zeroize::Zeroizing<[u8; MASTER_KEY_LEN]>>,
}

impl InMemoryMasterKeyProvider {
    pub fn random() -> Self {
        let key = MasterKey::generate();
        Self {
            key_bytes: std::sync::Mutex::new(zeroize::Zeroizing::new(*key.as_bytes())),
        }
    }
}

impl MasterKeyProvider for InMemoryMasterKeyProvider {
    fn load_or_create(&self) -> Result<MasterKey, VaultError> {
        let guard = self.key_bytes.lock().expect("in-memory key mutex poisoned");
        MasterKey::from_bytes(guard.as_slice())
    }

    fn describe(&self) -> String {
        "in-memory (ephemeral)".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_is_redacted() {
        let key = MasterKey::generate();
        assert_eq!(format!("{key:?}"), "MasterKey([REDACTED])");
    }

    #[test]
    fn file_provider_creates_then_reuses_key() {
        let dir = tempfile::tempdir().unwrap();
        let provider = FileMasterKeyProvider::new(dir.path().join("nested").join("master.key"));
        let first = provider.load_or_create().unwrap();
        let second = provider.load_or_create().unwrap();
        assert_eq!(first.as_bytes(), second.as_bytes());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(provider.path()).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn rejects_wrong_length() {
        assert!(matches!(
            MasterKey::from_bytes(&[0u8; 16]),
            Err(VaultError::InvalidKeyLength { found: 16, .. })
        ));
    }
}
