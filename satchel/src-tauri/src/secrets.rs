//! API key storage. On Windows the key lives in Windows Credential Manager (generic credential
//! "anthropic-api-key.Satchel"). It is never written to the database, a config file, or logs.
//!
//! On other platforms (only used for development) an in-memory store is used, so the key is
//! forgotten when the app quits.

use std::fmt;

use keyring_core::{Entry, Error as KeyringError};
use serde::Serialize;

use crate::error::{AppError, AppResult};

const SERVICE: &str = "Satchel";
const ACCOUNT: &str = "anthropic-api-key";

/// An API key. `Debug` is redacted so it can't leak into logs by accident.
#[derive(Clone)]
pub struct ApiKey(String);

impl ApiKey {
    pub fn parse(raw: &str) -> AppResult<Self> {
        let key = raw.trim();
        if key.is_empty() {
            return Err(AppError::invalid("Paste your API key first."));
        }
        if key.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(AppError::invalid("The API key contains spaces or line breaks. Copy it again from the Claude Console."));
        }
        if key.len() < 20 || key.len() > 512 {
            return Err(AppError::invalid("That doesn't look like a complete API key. Copy it again from the Claude Console."));
        }
        Ok(Self(key.to_string()))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    /// "sk-ant-…a1b2": enough to recognise which key is saved without revealing it.
    pub fn hint(&self) -> String {
        let tail: String = self.0.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
        let head: String = self.0.chars().take(7).collect();
        format!("{head}…{tail}")
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey(***)")
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyStoreInfo {
    /// Human-readable store name shown in Settings.
    pub name: &'static str,
    /// False for the development-only in-memory store.
    pub persistent: bool,
    pub error: Option<String>,
}

/// Installs the platform credential store. Call once at startup, before any other function here.
pub fn init_store() -> KeyStoreInfo {
    #[cfg(windows)]
    {
        match windows_native_keyring_store::Store::new() {
            Ok(store) => {
                keyring_core::set_default_store(store);
                KeyStoreInfo { name: "Windows Credential Manager", persistent: true, error: None }
            }
            Err(e) => KeyStoreInfo { name: "Windows Credential Manager", persistent: true, error: Some(e.to_string()) },
        }
    }
    #[cfg(not(windows))]
    {
        tracing::warn!("non-Windows build: the API key is kept in memory only and is forgotten on exit");
        match keyring_core::mock::Store::new() {
            Ok(store) => {
                keyring_core::set_default_store(store);
                KeyStoreInfo { name: "memory for this session", persistent: false, error: None }
            }
            Err(e) => KeyStoreInfo { name: "memory for this session", persistent: false, error: Some(e.to_string()) },
        }
    }
}

fn entry() -> AppResult<Entry> {
    Entry::new(SERVICE, ACCOUNT).map_err(|e| AppError::Secrets(e.to_string()))
}

pub fn load_api_key() -> AppResult<Option<ApiKey>> {
    match entry()?.get_password() {
        Ok(raw) => Ok(ApiKey::parse(&raw).ok()),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Secrets(e.to_string())),
    }
}

pub fn save_api_key(key: &ApiKey) -> AppResult<()> {
    entry()?.set_password(key.expose()).map_err(|e| AppError::Secrets(e.to_string()))
}

pub fn delete_api_key() -> AppResult<()> {
    match entry()?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Secrets(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_garbage_and_redacts() {
        assert!(ApiKey::parse("   ").is_err());
        assert!(ApiKey::parse("sk-ant-test with spaces in it xxx").is_err());
        assert!(ApiKey::parse("short").is_err());
        let key = ApiKey::parse("  sk-ant-test-abcdefghijklmnopqrstuvwxyz-WXYZ \n").unwrap();
        assert_eq!(key.expose(), "sk-ant-test-abcdefghijklmnopqrstuvwxyz-WXYZ");
        assert_eq!(key.hint(), "sk-ant-…WXYZ");
        assert_eq!(format!("{key:?}"), "ApiKey(***)");
    }
}
