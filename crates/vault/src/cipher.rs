//! Sealing and opening secrets with XChaCha20-Poly1305.

use std::fmt;

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rand::RngCore;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::error::VaultError;
use crate::master_key::{MasterKey, MasterKeyProvider};

/// XChaCha20 uses a 192-bit nonce, large enough to draw randomly without bookkeeping.
pub const NONCE_LEN: usize = 24;

/// Ciphertext plus the nonce that was used to produce it.
///
/// This is what gets persisted. It is safe to `Debug`, log and serialize:
/// without the master key it reveals nothing about the plaintext beyond its length.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedSecret {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}

impl fmt::Debug for EncryptedSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EncryptedSecret")
            .field("ciphertext_len", &self.ciphertext.len())
            .field("nonce_len", &self.nonce.len())
            .finish()
    }
}

/// Encrypts and decrypts secrets with a master key obtained from a [`MasterKeyProvider`].
pub struct Vault {
    cipher: XChaCha20Poly1305,
    key_location: String,
}

impl Vault {
    /// Open the vault, loading (or creating) the master key through `provider`.
    pub fn open(provider: &dyn MasterKeyProvider) -> Result<Self, VaultError> {
        let key = provider.load_or_create()?;
        Ok(Self::from_master_key(&key, provider.describe()))
    }

    fn from_master_key(key: &MasterKey, key_location: String) -> Self {
        let cipher = XChaCha20Poly1305::new(key.as_bytes().into());
        Self { cipher, key_location }
    }

    /// Where the master key is stored, for status output. Never includes key material.
    pub fn key_location(&self) -> &str {
        &self.key_location
    }

    /// Seal `plaintext`, binding it to `aad` (associated data).
    ///
    /// Callers pass a stable identifier for the record (for example the secret row id)
    /// as `aad` so a ciphertext cannot be silently moved to a different record.
    pub fn encrypt(&self, plaintext: &SecretString, aad: &[u8]) -> Result<EncryptedSecret, VaultError> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(
                nonce,
                Payload {
                    msg: plaintext.expose_secret().as_bytes(),
                    aad,
                },
            )
            .map_err(|_| VaultError::Encrypt)?;

        Ok(EncryptedSecret {
            ciphertext,
            nonce: nonce_bytes.to_vec(),
        })
    }

    /// Open a sealed secret. `aad` must match what was passed to [`Vault::encrypt`].
    pub fn decrypt(&self, sealed: &EncryptedSecret, aad: &[u8]) -> Result<SecretString, VaultError> {
        if sealed.nonce.len() != NONCE_LEN {
            return Err(VaultError::InvalidNonceLength {
                found: sealed.nonce.len(),
                expected: NONCE_LEN,
            });
        }
        let nonce = XNonce::from_slice(&sealed.nonce);
        let mut plaintext = self
            .cipher
            .decrypt(
                nonce,
                Payload {
                    msg: &sealed.ciphertext,
                    aad,
                },
            )
            .map_err(|_| VaultError::Decrypt)?;

        match String::from_utf8(std::mem::take(&mut plaintext)) {
            Ok(s) => Ok(SecretString::from(s)),
            Err(err) => {
                let mut bytes = err.into_bytes();
                zeroize::Zeroize::zeroize(&mut bytes);
                Err(VaultError::InvalidUtf8)
            }
        }
    }
}

impl fmt::Debug for Vault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vault")
            .field("key_location", &self.key_location)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::master_key::InMemoryMasterKeyProvider;

    fn vault() -> Vault {
        Vault::open(&InMemoryMasterKeyProvider::random()).unwrap()
    }

    #[test]
    fn round_trip() {
        let v = vault();
        let sealed = v
            .encrypt(&SecretString::from("sk-live-123"), b"secret-id")
            .unwrap();
        let opened = v.decrypt(&sealed, b"secret-id").unwrap();
        assert_eq!(opened.expose_secret(), "sk-live-123");
    }

    #[test]
    fn ciphertext_does_not_contain_plaintext() {
        let v = vault();
        let sealed = v.encrypt(&SecretString::from("hunter2-hunter2"), b"id").unwrap();
        let hay = String::from_utf8_lossy(&sealed.ciphertext);
        assert!(!hay.contains("hunter2"));
        assert_ne!(sealed.ciphertext.as_slice(), b"hunter2-hunter2");
    }

    #[test]
    fn nonce_is_unique_per_call() {
        let v = vault();
        let a = v.encrypt(&SecretString::from("x"), b"id").unwrap();
        let b = v.encrypt(&SecretString::from("x"), b"id").unwrap();
        assert_ne!(a.nonce, b.nonce);
        assert_ne!(a.ciphertext, b.ciphertext);
    }

    #[test]
    fn wrong_aad_fails() {
        let v = vault();
        let sealed = v.encrypt(&SecretString::from("x"), b"record-a").unwrap();
        assert!(matches!(
            v.decrypt(&sealed, b"record-b"),
            Err(VaultError::Decrypt)
        ));
    }

    #[test]
    fn wrong_key_fails() {
        let a = vault();
        let b = vault();
        let sealed = a.encrypt(&SecretString::from("x"), b"id").unwrap();
        assert!(matches!(b.decrypt(&sealed, b"id"), Err(VaultError::Decrypt)));
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let v = vault();
        let mut sealed = v.encrypt(&SecretString::from("x"), b"id").unwrap();
        sealed.ciphertext[0] ^= 0xff;
        assert!(matches!(v.decrypt(&sealed, b"id"), Err(VaultError::Decrypt)));
    }

    #[test]
    fn debug_never_prints_material() {
        let v = vault();
        let sealed = v.encrypt(&SecretString::from("topsecret"), b"id").unwrap();
        let dbg = format!("{sealed:?} {v:?}");
        assert!(!dbg.contains("topsecret"));
        assert!(dbg.contains("ciphertext_len"));
    }
}
