use crate::core::paths::AppPaths;
use crate::core::process;
use crate::core::session;
use crate::core::session::file_hash_debug;
use crate::core::state;
use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::{Emitter, State};
use tauri_plugin_store::StoreExt;

/// The store file name used by tauri-plugin-store.
pub const STORE_FILE: &str = "settings.json";

/// Append a timestamped line to the switch log file.
fn slog(log_path: &Path, msg: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
    {
        let _ = writeln!(f, "[{}] {}", chrono_now(), msg);
    }
    log::info!("[switch] {}", msg);
}

/// Simple timestamp without external crates.
fn chrono_now() -> String {
    use std::time::SystemTime;
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => format!("T+{}s", d.as_secs()),
        Err(_) => "T+?".to_string(),
    }
}

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

    // Set up file logging for this switch operation
    let log_path = paths.profiles_dir.join("switch.log");
    // Clear previous log
    let _ = std::fs::write(&log_path, "");

    slog(&log_path, &format!("=== Switch to {} STARTED ===", id));
    slog(&log_path, &format!(
        "Pre-switch auth-v2.dat hash: {}",
        file_hash_debug(&paths.auth_v2_dat)
    ));
    slog(&log_path, &format!(
        "Pre-switch auth.dat hash: {}",
        file_hash_debug(&paths.auth_dat)
    ));

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

    slog(&log_path, &format!(
        "Target backup auth-v2.dat hash: {}",
        file_hash_debug(&profile_dir.join("auth-v2.dat"))
    ));
    slog(&log_path, &format!(
        "Target backup auth.dat hash: {}",
        file_hash_debug(&profile_dir.join("auth.dat"))
    ));

    // Step 1: Auto-save current account before switching (prevent data loss)
    emit_progress(&app_handle, "正在保存当前账号...", 1, total_steps);
    if let Some(current_uid) = crate::core::status::get_current_user_id(&paths) {
        slog(&log_path, &format!("Current userId from .status.json: {}", current_uid));
        // Find the currently active account
        if let Some(current_account) = state_data.accounts.iter().find(|a| a.user_id.as_deref() == Some(&current_uid)) {
            if current_account.id != id {
                slog(&log_path, &format!("Auto-saving current account: {}", current_account.id));
                match session::save_auth_data(&paths, &current_account.id) {
                    Ok(_) => {
                        slog(&log_path, &format!(
                            "Auto-save OK. Saved auth-v2.dat hash: {}",
                            file_hash_debug(&paths.profile_dir(&current_account.id).join("auth-v2.dat"))
                        ));
                        // Mark as saved
                        let mut state_copy = state::read_state(&paths)?;
                        if let Some(acc) = state_copy.accounts.iter_mut().find(|a| a.id == current_account.id) {
                            acc.saved = true;
                        }
                        state::write_state(&paths, &state_copy)?;
                    }
                    Err(e) => {
                        slog(&log_path, &format!("Auto-save FAILED: {}", e));
                    }
                }
            } else {
                slog(&log_path, "Current account is same as target, skipping auto-save");
            }
        } else {
            slog(&log_path, "No matching account found for current userId, skipping auto-save");
        }
    } else {
        slog(&log_path, "Could not detect current userId from .status.json");
    }

    // Step 2: Kill the app
    emit_progress(&app_handle, "正在关闭 QoderWork CN...", 2, total_steps);
    slog(&log_path, "Killing QoderWork CN...");
    tokio::task::spawn_blocking(|| process::kill_app())
        .await
        .map_err(|e| AppError::Process(format!("Kill task panicked: {}", e)))??;

    slog(&log_path, &format!(
        "Post-kill: is_app_running={}",
        process::is_app_running()
    ));

    // Extra safety delay for file handle release
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Step 3: Clear current session
    emit_progress(&app_handle, "正在清除当前会话...", 3, total_steps);
    slog(&log_path, &format!(
        "Pre-clear auth-v2.dat hash: {}",
        file_hash_debug(&paths.auth_v2_dat)
    ));
    session::clear_session(&paths)?;

    // Verify clear
    slog(&log_path, &format!(
        "Post-clear auth-v2.dat exists: {}",
        paths.auth_v2_dat.exists()
    ));
    slog(&log_path, &format!(
        "Post-clear auth.dat exists: {}",
        paths.auth_dat.exists()
    ));
    // Check root dirs
    for dir_name in &["Network", "Local Storage", "Cache"] {
        let dir = paths.app_data_dir.join(dir_name);
        slog(&log_path, &format!(
            "Post-clear root/{}/ exists: {}",
            dir_name,
            dir.exists()
        ));
    }

    if paths.auth_v2_dat.exists() {
        slog(&log_path, "auth-v2.dat STILL EXISTS after clear — retrying...");
        tokio::time::sleep(Duration::from_millis(1000)).await;
        session::clear_session(&paths)?;
        if paths.auth_v2_dat.exists() {
            slog(&log_path, "FATAL: auth-v2.dat still exists after retry");
            return Err(AppError::Session(
                "无法清除会话数据，QoderWork CN 可能仍在运行。请手动关闭 QoderWork CN 后重试。".to_string(),
            ));
        }
    }
    slog(&log_path, "Clear verified: auth-v2.dat removed");
    slog(&log_path, &format!(
        "Post-clear .status.json exists: {}",
        paths.status_file.exists()
    ));

    // Step 4: Restore target account session
    emit_progress(&app_handle, "正在恢复账号数据...", 4, total_steps);
    slog(&log_path, &format!("Restoring session for account: {}", id));
    session::restore_auth_data(&paths, &id)?;

    // Verify restore
    let backup_auth_v2 = profile_dir.join("auth-v2.dat");
    if backup_auth_v2.exists() {
        if !paths.auth_v2_dat.exists() {
            slog(&log_path, "auth-v2.dat NOT restored — retrying...");
            tokio::time::sleep(Duration::from_millis(1000)).await;
            session::restore_auth_data(&paths, &id)?;
        }
        let backup_hash = file_hash_debug(&backup_auth_v2);
        let restored_hash = file_hash_debug(&paths.auth_v2_dat);
        slog(&log_path, &format!(
            "Post-restore auth-v2.dat: backup={}, restored={}, match={}",
            backup_hash,
            restored_hash,
            backup_hash == restored_hash
        ));

        let backup_auth_dat = profile_dir.join("auth.dat");
        slog(&log_path, &format!(
            "Post-restore auth.dat: backup={}, restored={}, match={}",
            file_hash_debug(&backup_auth_dat),
            file_hash_debug(&paths.auth_dat),
            file_hash_debug(&backup_auth_dat) == file_hash_debug(&paths.auth_dat)
        ));
    }

    // Update active account in state
    let mut state_data = state::read_state(&paths)?;
    state_data.active = Some(id.clone());
    state::write_state(&paths, &state_data)?;

    // Step 5: Relaunch the app
    emit_progress(&app_handle, "正在启动 QoderWork CN...", 5, total_steps);
    let exe_path = get_exe_path(&app_handle)?;
    slog(&log_path, &format!("Launching app: {:?}", exe_path));
    process::launch_app(&exe_path)?;

    // Verify the app actually started and got the right user
    tokio::time::sleep(Duration::from_millis(5000)).await;
    let running = process::is_app_running();
    slog(&log_path, &format!("Post-launch: is_app_running={}", running));
    slog(&log_path, &format!(
        "Post-launch auth-v2.dat hash: {} (backup was: {})",
        file_hash_debug(&paths.auth_v2_dat),
        file_hash_debug(&backup_auth_v2)
    ));

    // Check .status.json to verify the app authenticated as the correct user
    let expected_uid = account.user_id.as_deref().unwrap_or("(unknown)");
    match crate::core::status::get_current_user_id(&paths) {
        Some(actual_uid) => {
            slog(&log_path, &format!(
                "Post-launch userId: {} (expected: {}), match={}",
                actual_uid, expected_uid, actual_uid == expected_uid
            ));
            if actual_uid != expected_uid {
                slog(&log_path, "WARNING: userId mismatch! Auth files may be stale or expired.");
                slog(&log_path, "Please log into this account in QoderWork CN and click 'Save Current' to refresh the backup.");
            }
        }
        None => {
            slog(&log_path, "Post-launch: .status.json not yet written (app may still be authenticating)");
        }
    }
    slog(&log_path, "=== Switch COMPLETE ===");

    if !running {
        return Err(AppError::Process(
            "QoderWork CN 启动失败，请手动打开应用并重试。".to_string(),
        ));
    }

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
