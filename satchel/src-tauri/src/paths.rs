//! Where Satchel keeps its data.
//!
//! The library (SQLite db + copied originals) lives in the *data folder*. By default that is the
//! per-user local app data folder (`%LOCALAPPDATA%\app.satchel.desktop` on Windows; "Local", not
//! "Roaming", because the library can grow to gigabytes). The user can move it; the chosen location
//! is stored in a tiny `location.json` in the config folder, because it can't live inside the
//! database it points to.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};

pub const DB_FILE: &str = "satchel.db";
pub const FILES_DIR: &str = "files";
const LOCATION_FILE: &str = "location.json";

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub default_data_dir: PathBuf,
    pub location_file: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct LocationFile {
    data_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve(app: &AppHandle) -> AppResult<Self> {
        let default_data_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| AppError::internal(format!("Couldn't find the app data folder: {e}")))?;
        let config_dir = app
            .path()
            .app_config_dir()
            .map_err(|e| AppError::internal(format!("Couldn't find the app config folder: {e}")))?;
        fs::create_dir_all(&config_dir)?;
        let location_file = config_dir.join(LOCATION_FILE);

        let data_dir = match read_location(&location_file) {
            Some(custom) => {
                // A moved library that has gone missing (e.g. an unplugged USB drive) must not be
                // silently replaced by a fresh, empty one.
                if !custom.join(DB_FILE).is_file() {
                    return Err(AppError::invalid(format!(
                        "Satchel's library folder is missing: {}. Reconnect the drive it's on, or delete {} to start over in the default location.",
                        custom.display(),
                        location_file.display()
                    )));
                }
                custom
            }
            None => default_data_dir.clone(),
        };

        fs::create_dir_all(data_dir.join(FILES_DIR))?;
        Ok(Self { data_dir, default_data_dir, location_file })
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join(DB_FILE)
    }

    pub fn files_dir(&self) -> PathBuf {
        self.data_dir.join(FILES_DIR)
    }

    pub fn is_custom(&self) -> bool {
        self.data_dir != self.default_data_dir
    }

    /// Points future launches at `data_dir`. Written atomically (temp file + rename).
    pub fn write_location(&self, data_dir: &Path) -> AppResult<()> {
        let json = serde_json::to_vec_pretty(&LocationFile { data_dir: data_dir.to_path_buf() })
            .map_err(|e| AppError::internal(e.to_string()))?;
        let tmp = self.location_file.with_extension("json.tmp");
        fs::write(&tmp, json)?;
        fs::rename(&tmp, &self.location_file)?;
        Ok(())
    }
}

fn read_location(file: &Path) -> Option<PathBuf> {
    let bytes = fs::read(file).ok()?;
    let parsed: LocationFile = serde_json::from_slice(&bytes).ok()?;
    parsed.data_dir.is_absolute().then_some(parsed.data_dir)
}

/// Recursively copies `from` into `to` (which must exist). Used when moving the library.
pub fn copy_dir_recursive(from: &Path, to: &Path) -> std::io::Result<u64> {
    let mut copied = 0;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            fs::create_dir_all(&target)?;
            copied += copy_dir_recursive(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target)?;
            copied += 1;
        }
    }
    Ok(copied)
}
