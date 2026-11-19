use serde::{Deserialize, Serialize};
use tauri::State;

use crate::ai::models::{self, DEFAULT_MODEL};
use crate::ai::ConnectionTest;
use crate::db::settings::{self as settings_db, Settings};
use crate::db::usage::{self, UsageSummary};
use crate::db::migrations;
use crate::error::{AppError, AppResult};
use crate::secrets::{self, ApiKey, KeyStoreInfo};
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyStatus {
    pub present: bool,
    /// e.g. "sk-ant-…a1b2"
    pub hint: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub version: &'static str,
    pub data_dir: String,
    pub default_data_dir: String,
    pub is_custom_data_dir: bool,
    pub db_path: String,
    pub schema_version: i64,
    pub key_store: KeyStoreInfo,
    pub api_key: KeyStatus,
    pub settings: Settings,
}

fn key_status(state: &AppState) -> KeyStatus {
    let hint = state.ai.key_hint();
    KeyStatus { present: hint.is_some(), hint }
}

#[tauri::command]
pub async fn get_app_status(state: State<'_, AppState>) -> AppResult<AppStatus> {
    let (settings, schema_version) = state
        .db
        .call(|conn| Ok((settings_db::load(conn)?, migrations::current_version(conn)?)))
        .await?;
    Ok(AppStatus {
        version: env!("CARGO_PKG_VERSION"),
        data_dir: state.paths.data_dir.display().to_string(),
        default_data_dir: state.paths.default_data_dir.display().to_string(),
        is_custom_data_dir: state.paths.is_custom(),
        db_path: state.paths.db_path().display().to_string(),
        schema_version,
        key_store: state.key_store.clone(),
        api_key: key_status(&state),
        settings,
    })
}

#[tauri::command]
pub async fn save_settings(state: State<'_, AppState>, settings: Settings) -> AppResult<Settings> {
    let settings = settings.normalized()?;
    let saved = settings.clone();
    state.db.call(move |conn| settings_db::save(conn, &saved)).await?;
    Ok(settings)
}

#[tauri::command]
pub async fn set_api_key(state: State<'_, AppState>, key: String) -> AppResult<KeyStatus> {
    let key = ApiKey::parse(&key)?;
    let to_store = key.clone();
    tauri::async_runtime::spawn_blocking(move || secrets::save_api_key(&to_store))
        .await
        .map_err(|e| AppError::internal(e.to_string()))??;
    state.ai.set_key(Some(key));
    tracing::info!("API key saved to the credential store");
    Ok(key_status(&state))
}

#[tauri::command]
pub async fn delete_api_key(state: State<'_, AppState>) -> AppResult<KeyStatus> {
    tauri::async_runtime::spawn_blocking(secrets::delete_api_key)
        .await
        .map_err(|e| AppError::internal(e.to_string()))??;
    state.ai.set_key(None);
    tracing::info!("API key removed from the credential store");
    Ok(key_status(&state))
}

#[tauri::command]
pub async fn test_connection(state: State<'_, AppState>) -> AppResult<ConnectionTest> {
    let model = state.db.call(|conn| Ok(settings_db::load(conn)?.model)).await?;
    Ok(state.ai.test_connection(&model).await?)
}

#[tauri::command]
pub async fn get_usage_summary(state: State<'_, AppState>) -> AppResult<UsageSummary> {
    state.db.call(|conn| usage::summary(conn)).await
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOption {
    pub id: &'static str,
    pub name: &'static str,
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    pub is_default: bool,
}

#[tauri::command]
pub fn list_models() -> Vec<ModelOption> {
    models::KNOWN_MODELS
        .iter()
        .map(|m| ModelOption {
            id: m.id,
            name: m.name,
            input_per_mtok: m.pricing.input,
            output_per_mtok: m.pricing.output,
            is_default: m.id == DEFAULT_MODEL,
        })
        .collect()
}
