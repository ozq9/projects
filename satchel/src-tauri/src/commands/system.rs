use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use rusqlite::params;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::error::{AppError, AppResult};
use crate::paths::{copy_dir_recursive, DB_FILE, FILES_DIR};
use crate::state::{AppState, StartupError};

/// `Some(message)` if the app failed to start (the UI then shows it full-screen).
#[tauri::command]
pub fn get_startup_error(startup: State<'_, StartupError>) -> Option<String> {
    startup.0.clone()
}

#[tauri::command]
pub fn open_data_dir(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    app.opener()
        .open_path(state.paths.data_dir.display().to_string(), None::<&str>)
        .map_err(|e| AppError::internal(format!("Couldn't open the folder: {e}")))
}

/// Opens the Claude Console page where API keys are created.
#[tauri::command]
pub fn open_api_keys_page(app: AppHandle) -> AppResult<()> {
    app.opener()
        .open_url("https://console.anthropic.com/settings/keys", None::<&str>)
        .map_err(|e| AppError::internal(format!("Couldn't open the browser: {e}")))
}

/// Native folder picker. `None` if the user cancelled.
#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> AppResult<Option<String>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Choose an empty folder for your Satchel library")
        .pick_folder(move |picked| {
            let _ = tx.send(picked);
        });
    let picked = rx.await.map_err(|_| AppError::internal("The folder picker closed unexpectedly."))?;
    Ok(picked.and_then(|p| p.into_path().ok()).map(|p| p.display().to_string()))
}

/// Copies the library into `target` (an empty folder), points Satchel at it, and restarts.
/// The old folder is left untouched as a backup.
#[tauri::command]
pub async fn move_data_dir(app: AppHandle, state: State<'_, AppState>, target: String) -> AppResult<()> {
    let target = PathBuf::from(target.trim());
    if !target.is_absolute() {
        return Err(AppError::invalid("Choose a complete folder path, like D:\\School\\Satchel."));
    }
    fs::create_dir_all(&target)?;
    let current = fs::canonicalize(&state.paths.data_dir)?;
    let chosen = fs::canonicalize(&target)?;
    if chosen.starts_with(&current) || current.starts_with(&chosen) {
        return Err(AppError::invalid("Choose a folder outside the current library folder."));
    }
    if fs::read_dir(&target)?.next().is_some() {
        return Err(AppError::invalid("Choose an empty folder. This one already has files in it."));
    }

    let result = copy_library(&state, &target).await;
    if let Err(e) = result {
        // Leave the user's chosen folder as we found it (empty).
        let _ = fs::remove_file(target.join(DB_FILE));
        let _ = fs::remove_dir_all(target.join(FILES_DIR));
        return Err(e);
    }
    state.paths.write_location(&target)?;
    tracing::info!(to = %target.display(), "library moved; restarting");

    // Restart a moment later so this command's response reaches the UI first.
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(800)).await;
        app.restart();
    });
    Ok(())
}

async fn copy_library(state: &AppState, target: &std::path::Path) -> AppResult<()> {
    // VACUUM INTO writes a consistent snapshot even while the database is open.
    let db_target = target.join(DB_FILE).display().to_string();
    state
        .db
        .call(move |conn| {
            conn.execute("VACUUM INTO ?1", params![db_target])?;
            Ok(())
        })
        .await?;

    let from = state.paths.files_dir();
    let to = target.join(FILES_DIR);
    tauri::async_runtime::spawn_blocking(move || -> std::io::Result<u64> {
        fs::create_dir_all(&to)?;
        copy_dir_recursive(&from, &to)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))??;
    Ok(())
}
