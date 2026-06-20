use crate::core::paths::AppPaths;
use crate::core::process;
use crate::core::session;
use crate::core::state;
use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use tauri::{Emitter, State};
use tauri_plugin_store::StoreExt;

/// The store file name used by tauri-plugin-store.
pub const STORE_FILE: &str = "settings.json";

/// Progress event payload emitted during switch/save operations.
#[derive(Serialize, Clone, Debug)]
pub struct ProgressPayload {
    pub step: String,
    pub current: u32,
    pub total: u32,
}

/// Switch to a different account:
/// 1. Kill the running app
/// 2. Clear current session data
/// 3. Restore the target account's saved session
/// 4. Relaunch the app
#[tauri::command]
pub async fn switch_account(
    app_handle: tauri::AppHandle,
    paths: State<'_, AppPaths>,
    id: String,
) -> AppResult<()> {
    let total_steps: u32 = 4;

    // Validate account exists and has saved data
    let state_data = state::read_state(&paths)?;
    let account = state_data
        .accounts
        .iter()
        .find(|a| a.id == id)
        .ok_or_else(|| AppError::AccountNotFound(id.clone()))?;

    if !account.saved {
        return Err(AppError::Session(format!(
            "Account {} has no saved session data. Please log in and save first.",
            id
        )));
    }

    let profile_dir = paths.profile_dir(&id);
    if !profile_dir.exists() {
        return Err(AppError::Session(format!(
            "Profile directory missing for account {}: {:?}",
            id, profile_dir
        )));
    }

    // Step 1: Kill the app
    emit_progress(&app_handle, "正在关闭 QoderWork CN...", 1, total_steps);
    process::kill_app()?;
    thread::sleep(Duration::from_millis(500));

    // Step 2: Clear current session
    emit_progress(&app_handle, "正在清除当前会话...", 2, total_steps);
    session::clear_session(&paths)?;

    // Step 3: Restore target account session
    emit_progress(&app_handle, "正在恢复账号数据...", 3, total_steps);
    session::restore_auth_data(&paths, &id)?;

    // Update active account in state
    let mut state_data = state::read_state(&paths)?;
    state_data.active = Some(id.clone());
    state::write_state(&paths, &state_data)?;

    // Step 4: Relaunch the app
    emit_progress(&app_handle, "正在启动 QoderWork CN...", 4, total_steps);
    let exe_path = get_exe_path(&app_handle);
    process::launch_app(&exe_path)?;

    log::info!("Successfully switched to account {}", id);
    Ok(())
}

/// Save the current session for an account:
/// 1. Kill the running app (to ensure session data is flushed)
/// 2. Save session data
/// 3. Relaunch the app
#[tauri::command]
pub async fn save_account(
    app_handle: tauri::AppHandle,
    paths: State<'_, AppPaths>,
    id: String,
) -> AppResult<()> {
    let total_steps: u32 = 3;

    // Validate account exists
    let state_data = state::read_state(&paths)?;
    let _account = state_data
        .accounts
        .iter()
        .find(|a| a.id == id)
        .ok_or_else(|| AppError::AccountNotFound(id.clone()))?;

    // Step 1: Kill the app to ensure data is flushed
    emit_progress(&app_handle, "正在关闭 QoderWork CN...", 1, total_steps);
    process::kill_app()?;
    thread::sleep(Duration::from_millis(500));

    // Step 2: Save session data
    emit_progress(&app_handle, "正在保存账号数据...", 2, total_steps);
    session::save_auth_data(&paths, &id)?;

    // Mark account as saved and update userId from status
    let mut state_data = state::read_state(&paths)?;
    if let Some(account) = state_data.accounts.iter_mut().find(|a| a.id == id) {
        account.saved = true;
        // Try to detect and store the current userId
        if let Some(user_id) = crate::core::status::get_current_user_id(&paths) {
            account.user_id = Some(user_id);
        }
    }
    state::write_state(&paths, &state_data)?;

    // Step 3: Relaunch the app
    emit_progress(&app_handle, "正在启动 QoderWork CN...", 3, total_steps);
    let exe_path = get_exe_path(&app_handle);
    process::launch_app(&exe_path)?;

    log::info!("Successfully saved session for account {}", id);
    Ok(())
}

/// Emit a progress event to the frontend.
fn emit_progress(app_handle: &tauri::AppHandle, step: &str, current: u32, total: u32) {
    let payload = ProgressPayload {
        step: step.to_string(),
        current,
        total,
    };
    if let Err(e) = app_handle.emit("switch-progress", &payload) {
        log::warn!("Failed to emit progress event: {}", e);
    }
}

/// Resolve the executable path, preferring the stored path then falling back to auto-detection.
pub fn get_exe_path(app_handle: &tauri::AppHandle) -> PathBuf {
    // Try to get from store plugin
    if let Ok(store) = app_handle.store(STORE_FILE) {
        if let Some(val) = store.get("app_exe_path") {
            if let Some(path_str) = val.as_str() {
                let path = PathBuf::from(path_str);
                if path.exists() {
                    return path;
                }
            }
        }
    }

    // Fall back to auto-detection
    match crate::core::paths::find_app_exe() {
        Ok(path) => path,
        Err(e) => {
            log::error!("Failed to find app executable: {}", e);
            PathBuf::from("QoderWork CN")
        }
    }
}
