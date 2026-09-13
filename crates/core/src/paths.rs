use std::path::{Path, PathBuf};

use crate::error::{CoreError, Result};

/// Filesystem layout of the EnvFish data directory.
///
/// Default location:
/// - macOS: `~/Library/Application Support/envfish`
/// - Linux: `~/.local/share/envfish`
/// - Windows: `%APPDATA%\envfish`
///
/// Override with the `ENVFISH_HOME` environment variable (useful for tests and
/// for keeping several isolated vaults).
#[derive(Debug, Clone)]
pub struct Paths {
    root: PathBuf,
}

impl Paths {
    pub const ENV_OVERRIDE: &'static str = "ENVFISH_HOME";

    pub fn resolve() -> Result<Self> {
        if let Some(dir) = std::env::var_os(Self::ENV_OVERRIDE) {
            return Ok(Self::at(dir));
        }
        let base = dirs::data_dir().ok_or(CoreError::NoDataDir)?;
        Ok(Self::at(base.join("envfish")))
    }

    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn database(&self) -> PathBuf {
        self.root.join("envfish.db")
    }

    pub fn master_key(&self) -> PathBuf {
        self.root.join("master.key")
    }

    pub fn cli_state(&self) -> PathBuf {
        self.root.join("state.json")
    }

    /// Stable identifier for this data directory, used to name the keychain entry.
    pub fn profile(&self) -> String {
        let raw = self.root.to_string_lossy();
        raw.chars()
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
