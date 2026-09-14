//! # envenb-vault
//!
//! Secret encryption for EnvEnb.
//!
//! Responsibilities are split in two:
//!
//! - [`MasterKeyProvider`]: *where* the 256-bit master key lives
//!   (Phase 1: a `0600` file under the EnvEnb data directory; later: OS keychains).
//! - [`Vault`]: *how* secrets are sealed with that key (XChaCha20-Poly1305, AEAD).
//!
//! Nothing in this crate implements cryptography by hand; primitives come from the
//! RustCrypto `chacha20poly1305` crate. Plaintext values only ever exist as
//! [`secrecy::SecretString`] and are zeroized on drop.

mod cipher;
mod error;
#[cfg(feature = "keychain")]
mod keychain;
mod master_key;

pub use cipher::{EncryptedSecret, Vault};
pub use error::VaultError;
#[cfg(feature = "keychain")]
pub use keychain::KeychainMasterKeyProvider;
pub use master_key::{FileMasterKeyProvider, InMemoryMasterKeyProvider, MasterKey, MasterKeyProvider};

/// Re-exported so downstream crates use the same `SecretString` type.
pub use secrecy::{ExposeSecret, SecretString};
