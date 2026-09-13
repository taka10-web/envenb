use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};

/// Persisted CLI selection (`envfish use` / `envfish env`).
///
/// Stores identifiers only; never values.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CliState {
    pub current_project_id: Option<String>,
    pub current_environment_id: Option<String>,
}

impl CliState {
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(raw) => Ok(serde_json::from_str(&raw)?),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(CoreError::Io {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| CoreError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(path, raw).map_err(|source| CoreError::Io {
            path: path.to_path_buf(),
            source,
        })
    }
}
