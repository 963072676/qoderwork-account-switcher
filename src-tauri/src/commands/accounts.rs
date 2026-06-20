use crate::core::paths::AppPaths;
use crate::core::state::{self, Account};
use crate::core::status;
use crate::error::{AppError, AppResult};
use serde::Serialize;
use tauri::State;

/// Account with additional runtime status information.
#[derive(Serialize, Clone, Debug)]
pub struct AccountWithStatus {
    pub id: String,
    pub phone: String,
    pub label: String,
    pub user_id: Option<String>,
    pub saved: bool,
    /// True if this account's userId matches the currently detected userId.
    pub is_current: bool,
}

/// List all accounts with their current status.
#[tauri::command]
pub fn list_accounts(paths: State<'_, AppPaths>) -> AppResult<Vec<AccountWithStatus>> {
    let state = state::read_state(&paths)?;
    let current_user_id = status::get_current_user_id(&paths);

    let accounts_with_status: Vec<AccountWithStatus> = state
        .accounts
        .into_iter()
        .map(|acc| {
            let is_current = match (&acc.user_id, &current_user_id) {
                (Some(acc_uid), Some(current_uid)) => acc_uid == current_uid,
                _ => false,
            };

            AccountWithStatus {
                id: acc.id,
                phone: acc.phone,
                label: acc.label,
                user_id: acc.user_id,
                saved: acc.saved,
                is_current,
            }
        })
        .collect();

    Ok(accounts_with_status)
}

/// Add a new account to the state.
#[tauri::command]
pub fn add_account(
    paths: State<'_, AppPaths>,
    phone: String,
    label: String,
    user_id: Option<String>,
) -> AppResult<Account> {
    let mut state = state::read_state(&paths)?;

    // Check for duplicate phone
    if state.accounts.iter().any(|a| a.phone == phone) {
        return Err(AppError::AccountAlreadyExists(format!(
            "Account with phone {} already exists",
            phone
        )));
    }

    let id = state::generate_unique_id(&state.accounts);

    let account = Account {
        id: id.clone(),
        phone: phone.clone(),
        label,
        user_id,
        saved: false,
    };

    state.accounts.push(account.clone());
    state::write_state(&paths, &state)?;

    log::info!("Added account: id={}, phone={}", id, phone);
    Ok(account)
}

/// Delete an account and remove its saved profile data.
#[tauri::command]
pub fn delete_account(paths: State<'_, AppPaths>, id: String) -> AppResult<()> {
    let mut state = state::read_state(&paths)?;

    let initial_len = state.accounts.len();
    state.accounts.retain(|a| a.id != id);

    if state.accounts.len() == initial_len {
        return Err(AppError::AccountNotFound(format!(
            "Account with id {} not found",
            id
        )));
    }

    // Clear active reference if it was the deleted account
    if state.active.as_deref() == Some(&id) {
        state.active = None;
    }

    // Remove profile directory
    let profile_dir = paths.profile_dir(&id);
    if profile_dir.exists() {
        std::fs::remove_dir_all(&profile_dir).map_err(|e| {
            AppError::Session(format!(
                "Failed to remove profile directory {:?}: {}",
                profile_dir, e
            ))
        })?;
    }

    state::write_state(&paths, &state)?;

    log::info!("Deleted account: id={}", id);
    Ok(())
}
