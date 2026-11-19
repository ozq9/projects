//! Satchel backend.
//!
//! Modules (milestone 1 ships `ai`, `db`, `commands`; `ingest`, `ocr` and `search` arrive with
//! milestones 2 and 5):
//! - `ai`       – Claude Messages API client: retries, timeouts, structured JSON, usage/cost
//! - `db`       – SQLite connection, migrations, settings, usage accounting
//! - `commands` – the Tauri commands the frontend calls
//! - `secrets`  – API key in Windows Credential Manager
//! - `paths`    – data folder resolution and moving

pub mod ai;
pub mod commands;
pub mod db;
pub mod error;
pub mod paths;
pub mod secrets;
pub mod state;

use tauri::Manager;

use crate::ai::ClaudeClient;
use crate::db::Db;
use crate::error::AppResult;
use crate::paths::AppPaths;
use crate::state::{AppState, StartupError};

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,satchel_lib=debug"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

fn build_state(app: &tauri::AppHandle) -> AppResult<AppState> {
    let key_store = secrets::init_store();
    let paths = AppPaths::resolve(app)?;
    tracing::info!(data_dir = %paths.data_dir.display(), "opening library");
    let db = Db::open(&paths.db_path())?;

    let api_key = match secrets::load_api_key() {
        Ok(key) => key,
        Err(e) => {
            // Not fatal: the app still works for browsing; Settings shows the problem.
            tracing::error!(error = %e, "couldn't read the API key from the credential store");
            None
        }
    };
    let ai = ClaudeClient::new(db.clone(), api_key)?;
    Ok(AppState { db, ai, paths, key_store })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            match build_state(app.handle()) {
                Ok(state) => {
                    app.manage(state);
                    app.manage(StartupError(None));
                }
                Err(e) => {
                    tracing::error!(error = %e, "startup failed");
                    app.manage(StartupError(Some(e.to_string())));
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::get_startup_error,
            commands::system::open_data_dir,
            commands::system::pick_folder,
            commands::system::move_data_dir,
            commands::system::open_api_keys_page,
            commands::settings::get_app_status,
            commands::settings::save_settings,
            commands::settings::set_api_key,
            commands::settings::delete_api_key,
            commands::settings::test_connection,
            commands::settings::get_usage_summary,
            commands::settings::list_models,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Satchel");
}
