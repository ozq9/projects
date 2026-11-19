use crate::ai::ClaudeClient;
use crate::db::Db;
use crate::paths::AppPaths;
use crate::secrets::KeyStoreInfo;

/// Shared state, managed by Tauri and available to every command.
pub struct AppState {
    pub db: Db,
    pub ai: ClaudeClient,
    pub paths: AppPaths,
    pub key_store: KeyStoreInfo,
}

/// Always managed. Holds the reason startup failed (if it did), so the UI can show it instead
/// of a blank window.
pub struct StartupError(pub Option<String>);
