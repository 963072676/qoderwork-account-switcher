use crate::commands::switch::STORE_FILE;
use crate::core::paths::{self, AppPaths};
use crate::core::status;
use crate::error::AppResult;
use tauri::State;
use tauri_plugin_store::StoreExt;

/// Detect the currently logged-in account by reading .status.json.
/// Called "detect_current_account" to match the frontend invoke name.
#[tauri::command]
pub fn detect_current_account(paths: State<'_, AppPaths>) -> AppResult<Option<String>> {
    let user_id = status::get_current_user_id(&paths);
    Ok(user_id)
}

/// Get the configured application executable path from the store.
/// Returns an empty string if no custom path has been set.
#[tauri::command]
pub fn get_exe_path(app_handle: tauri::AppHandle) -> AppResult<String> {
    if let Ok(store) = app_handle.store(STORE_FILE) {
        if let Some(val) = store.get("app_exe_path") {
            if let Some(path_str) = val.as_str() {
                return Ok(path_str.to_string());
            }
        }
    }
    Ok(String::new())
}

/// Set the application executable path in the store.
/// This overrides the auto-detected path.
#[tauri::command]
pub fn set_exe_path(
    app_handle: tauri::AppHandle,
    path: String,
) -> AppResult<()> {
    let store = app_handle.store(STORE_FILE).map_err(|e| {
        crate::error::AppError::StateFile(format!("Failed to open store: {}", e))
    })?;

    store.set("app_exe_path", serde_json::json!(path));
    store.save().map_err(|e| {
        crate::error::AppError::StateFile(format!("Failed to save store: {}", e))
    })?;

    log::info!("App executable path set to: {}", path);
    Ok(())
}

/// Auto-detect the QoderWork CN executable path by scanning common install locations.
/// Returns the detected path, or an error if not found.
#[tauri::command]
pub fn auto_detect_exe() -> AppResult<String> {
    match paths::find_app_exe() {
        Ok(path) => {
            let path_str = path.to_string_lossy().to_string();
            log::info!("Auto-detected app executable: {}", path_str);
            Ok(path_str)
        }
        Err(e) => {
            log::warn!("Auto-detect failed: {}", e);
            Err(e)
        }
    }
}
