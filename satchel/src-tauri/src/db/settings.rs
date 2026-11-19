//! User settings stored as key/value rows. (The API key is NOT here; see `secrets`.)

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::ai::models::DEFAULT_MODEL;
use crate::error::{AppError, AppResult};

const KEY_MODEL: &str = "ai.model";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Claude model id used for every call, e.g. "claude-opus-5-5".
    pub model: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self { model: DEFAULT_MODEL.to_string() }
    }
}

impl Settings {
    /// Trims and validates user input.
    pub fn normalized(mut self) -> AppResult<Self> {
        self.model = self.model.trim().to_string();
        if self.model.is_empty() {
            return Err(AppError::invalid("Enter a model name, for example claude-opus-5-5."));
        }
        let valid = self.model.len() <= 100
            && self.model.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '@' | '/'));
        if !valid {
            return Err(AppError::invalid("Model names only contain letters, numbers, dashes and dots, like claude-opus-5-5."));
        }
        Ok(self)
    }
}

fn get(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM settings WHERE key = ?1", params![key], |row| row.get(0))
        .optional()?)
}

fn set(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn load(conn: &Connection) -> AppResult<Settings> {
    let defaults = Settings::default();
    Ok(Settings { model: get(conn, KEY_MODEL)?.unwrap_or(defaults.model) })
}

pub fn save(conn: &Connection, settings: &Settings) -> AppResult<()> {
    set(conn, KEY_MODEL, &settings.model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_then_roundtrip() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&mut conn).unwrap();
        assert_eq!(load(&conn).unwrap().model, DEFAULT_MODEL);
        let s = Settings { model: "  claude-sonnet-5-5 ".into() }.normalized().unwrap();
        save(&conn, &s).unwrap();
        assert_eq!(load(&conn).unwrap().model, "claude-sonnet-5-5");
        assert!(Settings { model: "".into() }.normalized().is_err());
        assert!(Settings { model: "claude opus; drop".into() }.normalized().is_err());
    }
}
