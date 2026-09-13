//! Master key stored in the OS credential store via the `keyring` crate:
//! macOS Keychain, Windows Credential Manager, Linux Secret Service.

use zeroize::Zeroize;

use crate::error::VaultError;
use crate::master_key::{MasterKey, MasterKeyProvider};

const SERVICE: &str = "app.envfish.desktop";

/// Keeps the 32-byte master key as a credential named `envfish/<profile>`.
///
/// Unlike the file provider, other processes running as the same user do not
/// get the key for free: the OS prompts or enforces per-app access control.
pub struct KeychainMasterKeyProvider {
    account: String,
}

impl KeychainMasterKeyProvider {
    /// `profile` distinguishes several vaults (e.g. per `ENVFISH_HOME`).
    pub fn new(profile: impl Into<String>) -> Self {
        Self {
            account: format!("master-key/{}", profile.into()),
        }
    }

    fn entry(&self) -> Result<keyring::Entry, VaultError> {
        // `keyring` v1 initialises the platform store on first use; a failure
        // surfaces here as `NoDefaultStore`.
        keyring::Entry::new(SERVICE, &self.account).map_err(|e| VaultError::Keychain(e.to_string()))
    }

    /// Store an existing key (used when migrating from the file backend).
    pub fn store(&self, key: &MasterKey) -> Result<(), VaultError> {
        self.entry()?
            .set_secret(key.as_bytes())
            .map_err(|e| VaultError::Keychain(e.to_string()))
    }

    pub fn delete(&self) -> Result<(), VaultError> {
        match self.entry()?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(VaultError::Keychain(e.to_string())),
        }
    }

    pub fn exists(&self) -> Result<bool, VaultError> {
        match self.entry()?.get_secret() {
            Ok(mut bytes) => {
                bytes.zeroize();
                Ok(true)
            }
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(VaultError::Keychain(e.to_string())),
        }
    }
}

impl MasterKeyProvider for KeychainMasterKeyProvider {
    fn load_or_create(&self) -> Result<MasterKey, VaultError> {
        let entry = self.entry()?;
        match entry.get_secret() {
            Ok(mut bytes) => {
                let key = MasterKey::from_bytes(&bytes);
                bytes.zeroize();
                key
            }
            Err(keyring::Error::NoEntry) => {
                let key = MasterKey::generate();
                entry
                    .set_secret(key.as_bytes())
                    .map_err(|e| VaultError::Keychain(e.to_string()))?;
                tracing::info!("generated new master key in OS keychain");
                Ok(key)
            }
            Err(e) => Err(VaultError::Keychain(e.to_string())),
        }
    }

    fn describe(&self) -> String {
        format!("os keychain ({SERVICE} / {})", self.account)
    }
}
