use crate::core::paths::AppPaths;
use crate::core::process;
use crate::core::session;
use crate::core::state;
use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::path::PathBuf;
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
/// 1. Auto-save current account's session (prevent data loss)
/// 2. Kill the running app
/// 3. Clear current session data
/// 4. Restore the target account's saved session
/// 5. Relaunch the app
#[tauri::command]
pub async fn switch_account(
    app_handle: tauri::AppHandle,
    paths: State<'_, AppPaths>,
    id: String,
) -> AppResult<()> {
    let total_steps: u32 = 5;

    // Validate account exists and has saved data
    let state_data = state::read_state(&paths)?;
    let account = state_data
        .accounts
        .iter()
        .find(|a| a.id == id)
        .ok_or_else(|| AppError::AccountNotFound(id.clone()))?;

    if !account.saved {
        return Err(AppError::Session(format!(
            "账号「{}」尚未保存会话数据。请先登录该账号，然后点击「保存当前」。",
            account.label
        )));
    }

    let profile_dir = paths.profile_dir(&id);
    if !profile_dir.exists() {
        return Err(AppError::Session(format!(
            "账号「{}」的会话数据目录缺失: {:?}",
            account.label, profile_dir
        )));
    }

    // Step 1: Auto-save current account before switching (prevent data loss)
    emit_progress(&app_handle, "正在保存当前账号...", 1, total_steps);
    if let Some(current_uid) = crate::core::status::get_current_user_id(&paths) {
        // Find the currently active account
        if let Some(current_account) = state_data.accounts.iter().find(|a| a.user_id.as_deref() == Some(&current_uid)) {
            if current_account.id != id {
                log::info!("[switch] Auto-saving current account {} before switching", current_account.id);
                match session::save_auth_data(&paths, &current_account.id) {
                    Ok(_) => {
                        // Mark as saved
                        let mut state_copy = state::read_state(&paths)?;
                        if let Some(acc) = state_copy.accounts.iter_mut().find(|a| a.id == current_account.id) {
                            acc.saved = true;
                        }
                        state::write_state(&paths, &state_copy)?;
                        log::info!("[switch] Successfully saved current account {}", current_account.id);
                    }
                    Err(e) => {
                        log::warn!("[switch] Failed to auto-save current account {}: {}", current_account.id, e);
                        // Don't block the switch — just warn
                    }
                }
            }
        }
    }

    // Step 2: Kill the app (wait_for_exit verifies all processes exited)
    emit_progress(&app_handle, "正在关闭 QoderWork CN...", 2, total_steps);
    tokio::task::spawn_blocking(|| process::kill_app())
        .await
        .map_err(|e| AppError::Process(format!("Kill task panicked: {}", e)))??;

    // Extra safety delay for file handle release
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Step 3: Clear current session
    emit_progress(&app_handle, "正在清除当前会话...", 3, total_steps);
    session::clear_session(&paths)?;

    // Verify clear: auth-v2.dat should not exist
    if paths.auth_v2_dat.exists() {
        log::warn!("[switch] auth-v2.dat still exists after clear — file may be locked");
        // Retry once after a brief delay
        tokio::time::sleep(Duration::from_millis(1000)).await;
        session::clear_session(&paths)?;
        if paths.auth_v2_dat.exists() {
            return Err(AppError::Session(
                "无法清除会话数据，QoderWork CN 可能仍在运行。请手动关闭 QoderWork CN 后重试。".to_string(),
            ));
        }
    }

    // Step 4: Restore target account session
    emit_progress(&app_handle, "正在恢复账号数据...", 4, total_steps);
    session::restore_auth_data(&paths, &id)?;

    // Verify restore: auth-v2.dat should now exist (if the backup had it)
    let backup_auth_v2 = profile_dir.join("auth-v2.dat");
    if backup_auth_v2.exists() && !paths.auth_v2_dat.exists() {
        log::warn!("[switch] auth-v2.dat was not restored — retrying");
        tokio::time::sleep(Duration::from_millis(1000)).await;
        session::restore_auth_data(&paths, &id)?;
    }

    // Update active account in state
    let mut state_data = state::read_state(&paths)?;
    state_data.active = Some(id.clone());
    state::write_state(&paths, &state_data)?;

    // Step 5: Relaunch the app
    emit_progress(&app_handle, "正在启动 QoderWork CN...", 5, total_steps);
    let exe_path = get_exe_path(&app_handle)?;
    process::launch_app(&exe_path)?;

    // Verify the app actually started
    tokio::time::sleep(Duration::from_millis(2000)).await;
    if !process::is_app_running() {
        return Err(AppError::Process(
            "QoderWork CN 启动失败，请手动打开应用并重试。".to_string(),
        ));
    }

    log::info!("Successfully switched to account {}", id);
    Ok(())
}

/// Save the current session.
///
/// This detects which account is currently active (via .status.json userId),
/// finds the matching account in state.json, and saves its session data.
/// If no matching account is found but a user IS detected, auto-creates a new account.
/// If no user can be detected at all, returns a clear error message.
#[tauri::command]
pub async fn save_current_account(
    app_handle: tauri::AppHandle,
    paths: State<'_, AppPaths>,
) -> AppResult<()> {
    let total_steps: u32 = 3;

    // Detect current userId from .status.json
    let current_user_id = crate::core::status::get_current_user_id(&paths);

    // Read state
    let mut state_data = state::read_state(&paths)?;

    // Try to find which account this userId belongs to
    let account_id = if let Some(ref uid) = current_user_id {
        // Match by userId
        state_data
            .accounts
            .iter()
            .find(|a| a.user_id.as_deref() == Some(uid))
            .map(|a| a.id.clone())
    } else {
        None
    };

    // Fallback to the "active" field in state
    let account_id = account_id.or_else(|| state_data.active.clone());

    // If still no match, try to auto-create when user IS detected
    let account_id = match account_id {
        Some(id) => id,
        None => {
            if let Some(ref uid) = current_user_id {
                // Auto-create a new account entry for the detected user
                let new_id = state::generate_unique_id(&state_data.accounts);
                let account_count = state_data.accounts.len();
                let new_account = state::Account {
                    id: new_id.clone(),
                    phone: String::new(),
                    label: format!("账号{}", account_count + 1),
                    user_id: Some(uid.clone()),
                    saved: false,
                };
                state_data.accounts.push(new_account);
                state::write_state(&paths, &state_data)?;
                log::info!("[save] Auto-created account {} for detected user {}", new_id, uid);
                new_id
            } else {
                return Err(AppError::Session(
                    "未检测到当前登录的 QoderWork CN 账号。请先在 QoderWork CN 中登录，然后再点击「保存当前」。\n\n如果您已经登录，请尝试点击右上角的「检测」按钮刷新状态。"
                        .to_string(),
                ));
            }
        }
    };

    // Validate account exists
    let _account = state_data
        .accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| AppError::AccountNotFound(account_id.clone()))?;

    // Step 1: Kill the app to ensure data is flushed (wait_for_exit inside kill_app verifies)
    emit_progress(&app_handle, "正在关闭 QoderWork CN...", 1, total_steps);
    tokio::task::spawn_blocking(|| process::kill_app())
        .await
        .map_err(|e| AppError::Process(format!("Kill task panicked: {}", e)))??;

    // Extra safety delay for file handle release
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Step 2: Save session data
    emit_progress(&app_handle, "正在保存账号数据...", 2, total_steps);
    session::save_auth_data(&paths, &account_id)?;

    // Mark account as saved and update userId from status
    let mut state_data = state::read_state(&paths)?;
    if let Some(account) = state_data.accounts.iter_mut().find(|a| a.id == account_id) {
        account.saved = true;
        // Update userId from fresh detection
        if let Some(uid) = current_user_id {
            account.user_id = Some(uid);
        }
    }
    state_data.active = Some(account_id.clone());
    state::write_state(&paths, &state_data)?;

    // Step 3: Relaunch the app
    emit_progress(&app_handle, "正在启动 QoderWork CN...", 3, total_steps);
    let exe_path = get_exe_path(&app_handle)?;
    process::launch_app(&exe_path)?;

    // Verify the app actually started
    tokio::time::sleep(Duration::from_millis(2000)).await;
    if !process::is_app_running() {
        log::warn!("QoderWork CN may not have started after save operation");
    }

    log::info!("Successfully saved session for account {}", account_id);
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
pub fn get_exe_path(app_handle: &tauri::AppHandle) -> AppResult<PathBuf> {
    // Try to get from store plugin
    if let Ok(store) = app_handle.store(STORE_FILE) {
        if let Some(val) = store.get("app_exe_path") {
            if let Some(path_str) = val.as_str() {
                let path = PathBuf::from(path_str);
                if path.exists() {
                    return Ok(path);
                }
            }
        }
    }

    // Fall back to auto-detection
    crate::core::paths::find_app_exe()
}
