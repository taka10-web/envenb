use std::path::{Path, PathBuf};

use crate::error::{CoreError, Result};

/// Filesystem layout of the EnvEnb data directory.
///
/// Default location:
/// - macOS: `~/Library/Application Support/envenb`
/// - Linux: `~/.local/share/envenb`
/// - Windows: `%APPDATA%\envenb`
///
/// Override with the `ENVENB_HOME` environment variable (useful for tests and
/// for keeping several isolated vaults).
#[derive(Debug, Clone)]
pub struct Paths {
    root: PathBuf,
}

impl Paths {
    pub const ENV_OVERRIDE: &'static str = "ENVENB_HOME";
    /// Honoured for one more release so existing shells keep working.
    pub const LEGACY_ENV_OVERRIDE: &'static str = "ENVFISH_HOME";

    const DIR: &'static str = "envenb";
    const LEGACY_DIR: &'static str = "envfish";

    pub fn resolve() -> Result<Self> {
        if let Some(dir) = std::env::var_os(Self::ENV_OVERRIDE) {
            return Ok(Self::at(dir));
        }
        if let Some(dir) = std::env::var_os(Self::LEGACY_ENV_OVERRIDE) {
            return Ok(Self::at(dir));
        }
        let base = dirs::data_dir().ok_or(CoreError::NoDataDir)?;
        let root = base.join(Self::DIR);
        // The app was called EnvEnb until 2026-09. If a vault is still sitting
        // under the old name and nothing has been written under the new one,
        // move it across so the rename is invisible to the user.
        if !root.exists() {
            let legacy = base.join(Self::LEGACY_DIR);
            if legacy.is_dir() {
                std::fs::rename(&legacy, &root).map_err(|source| CoreError::Io { path: legacy, source })?;
            }
        }
        Ok(Self::at(root))
    }

    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn database(&self) -> PathBuf {
        let db = self.root.join("envenb.db");
        if !db.exists() {
            let legacy = self.root.join("envfish.db");
            if legacy.is_file() {
                return legacy;
            }
        }
        db
    }

    pub fn master_key(&self) -> PathBuf {
        self.root.join("master.key")
    }

    pub fn cli_state(&self) -> PathBuf {
        self.root.join("state.json")
    }

    /// Stable identifier for this data directory, used to name the keychain entry.
    pub fn profile(&self) -> String {
        Self::slug(&self.root)
    }

    /// The keychain entry name this vault used before the EnvEnb -> EnvEnb
    /// rename, so a keychain-backed master key can be carried across.
    pub fn legacy_profile(&self) -> Option<String> {
        let name = self.root.file_name()?;
        if name != Self::DIR {
            return None;
        }
        Some(Self::slug(&self.root.with_file_name(Self::LEGACY_DIR)))
    }

    fn slug(path: &Path) -> String {
        path.to_string_lossy()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .trim_matches('_')
            .to_string()
    }

    pub fn ensure_root(&self) -> Result<()> {
        std::fs::create_dir_all(&self.root).map_err(|source| CoreError::Io {
            path: self.root.clone(),
            source,
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.root, std::fs::Permissions::from_mode(0o700));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_profile_tracks_the_old_directory_name() {
        let paths = Paths::at("/home/someone/.local/share/envenb");
        assert_eq!(
            paths.legacy_profile().as_deref(),
            Some("home_someone_.local_share_envfish")
        );
    }

    #[test]
    fn a_custom_root_has_no_legacy_profile() {
        assert!(Paths::at("/tmp/scratch-vault").legacy_profile().is_none());
    }

    #[test]
    fn an_existing_database_keeps_its_old_filename() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::at(dir.path());
        assert_eq!(paths.database(), dir.path().join("envenb.db"));
        std::fs::write(dir.path().join("envfish.db"), b"").unwrap();
        assert_eq!(paths.database(), dir.path().join("envfish.db"));
    }
}
