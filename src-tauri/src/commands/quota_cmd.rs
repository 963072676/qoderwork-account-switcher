use crate::core::paths::AppPaths;
use crate::core::quota::{self, QuotaInfo};
use crate::core::state;
use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::collections::HashMap;
use tauri::State;

/// Tauri command: fetch quota/credit usage for the currently active account.
#[tauri::command]
pub async fn get_quota_usage(paths: State<'_, AppPaths>) -> AppResult<QuotaInfo> {
    quota::fetch_quota_usage(&paths).await
}

/// Tauri command: fetch quota for a specific saved account by its profile directory.
#[tauri::command]
pub async fn get_account_quota(
    paths: State<'_, AppPaths>,
    account_id: String,
) -> AppResult<QuotaInfo> {
    let profile_dir = paths.profile_dir(&account_id);
    if !profile_dir.exists() {
        return Err(AppError::Api(format!(
            "账号 {} 的存档目录不存在",
            account_id
        )));
    }
    quota::fetch_quota_for_profile(&profile_dir, &paths.app_data_dir).await
}

/// Compact quota summary for the all-accounts refresh.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QuotaSummary {
    /// Column 1: daily free 3.7MAX model remaining credits (addOnQuota.remaining)
    pub daily_free: Option<i64>,
    /// Column 2: other remaining credits (userQuota.remaining + org remaining)
    pub other_credits: Option<i64>,
    /// Column 3: whether the account has checked in today
    pub checked_in: Option<bool>,
    /// Column 4: subscription days remaining (from expiresAt)
    pub sub_days_remaining: Option<i64>,
    /// User type (e.g. "personal_professional", "personal_free")
    pub user_type: Option<String>,
    /// Whether quota is exceeded
    pub exceeded: Option<bool>,
}

/// Result of refreshing all quotas — includes both data and per-account errors.
#[derive(Serialize, Clone, Debug)]
pub struct AllQuotasResult {
    pub quotas: HashMap<String, Option<QuotaSummary>>,
    pub errors: HashMap<String, String>,
}

/// Tauri command: fetch quota for all saved accounts (in parallel).
/// Returns quota data and any errors encountered per account.
#[tauri::command]
pub async fn refresh_all_quotas(
    paths: State<'_, AppPaths>,
) -> AppResult<AllQuotasResult> {
    let state_data = state::read_state(&paths)?;
    let profiles_dir = paths.profiles_dir.clone();
    let app_data_dir = paths.app_data_dir.clone();

    log::info!("[quota] refresh_all_quotas: {} accounts in state", state_data.accounts.len());

    let mut handles = Vec::new();

    for account in &state_data.accounts {
        if !account.saved {
            log::info!("[quota] Skipping unsaved account: {}", account.id);
            continue;
        }
        let id = account.id.clone();
        let profile_dir = profiles_dir.join(&id);
        let app_data_dir = app_data_dir.clone();

        if !profile_dir.exists() {
            log::info!("[quota] Skipping account with missing profile dir: {}", id);
            continue;
        }

        log::info!("[quota] Spawning fetch for account: {}", id);
        let handle = tokio::spawn(async move {
            match quota::fetch_quota_for_profile(&profile_dir, &app_data_dir).await {
                Ok(info) => (id, Ok(to_summary(&info))),
                Err(e) => (id, Err(e.to_string())),
            }
        });
        handles.push(handle);
    }

    let mut quotas: HashMap<String, Option<QuotaSummary>> = HashMap::new();
    let mut errors: HashMap<String, String> = HashMap::new();

    for handle in handles {
        if let Ok((id, result)) = handle.await {
            match result {
                Ok(summary) => {
                    quotas.insert(id, summary);
                }
                Err(e) => {
                    log::warn!("Quota fetch failed for {}: {}", id, e);
                    quotas.insert(id.clone(), None);
                    errors.insert(id, e);
                }
            }
        }
    }

    Ok(AllQuotasResult { quotas, errors })
}

fn to_summary(info: &QuotaInfo) -> Option<QuotaSummary> {
    log::info!(
        "[quota] to_summary: user_type={:?}, expires_at={:?}, user_quota={:?}, add_on_quota={:?}, check_in={:?}, daily_free_model={:?}",
        info.user_type,
        info.expires_at,
        info.user_quota.as_ref().map(|q| q.remaining),
        info.add_on_quota.as_ref().map(|a| a.remaining),
        info.check_in.as_ref().and_then(|c| c.status.as_ref()),
        info.daily_free_model.as_ref().map(|m| (&m.model_name, m.remaining)),
    );

    // Column 1: daily free 3.7MAX model remaining uses (from activity API)
    let daily_free = info
        .daily_free_model
        .as_ref()
        .and_then(|m| m.remaining);

    // Column 2: other credits = plan quota + add-on + org resource package
    let other_credits = {
        let plan_remaining = info
            .user_quota
            .as_ref()
            .and_then(|q| q.remaining)
            .unwrap_or(0.0);
        let addon_remaining = info
            .add_on_quota
            .as_ref()
            .and_then(|a| a.remaining)
            .unwrap_or(0.0);
        let org_remaining = info
            .org_resource_package
            .as_ref()
            .filter(|o| o.available.unwrap_or(false))
            .and_then(|o| o.remaining)
            .unwrap_or(0.0);
        Some((plan_remaining + addon_remaining + org_remaining) as i64)
    };

    // Column 3: checked in today?
    let checked_in = info
        .check_in
        .as_ref()
        .and_then(|c| c.status.as_ref())
        .map(|s| s == "CLAIMED_TODAY");

    // Column 4: subscription days remaining (from expiresAt epoch ms)
    let sub_days_remaining = info.expires_at.and_then(|expires_ms| {
        if expires_ms <= 0.0 {
            return None;
        }
        // Sentinel guard: year > 9000 means "never expires"
        if expires_ms > 253402300799000.0 {
            return None;
        }
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0);
        let remaining_ms = expires_ms - now_ms;
        if remaining_ms <= 0.0 {
            Some(0)
        } else {
            const MS_PER_DAY: f64 = 86_400_000.0;
            Some((remaining_ms / MS_PER_DAY).ceil() as i64)
        }
    });

    Some(QuotaSummary {
        daily_free,
        other_credits,
        checked_in,
        sub_days_remaining,
        user_type: info.user_type.clone(),
        exceeded: info.is_quota_exceeded,
    })
}

/// Result of claiming check-in for all accounts.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClaimAllResult {
    /// Per-account claim results: account_id -> result string ("CLAIMED" / "ALREADY_CLAIMED")
    pub results: HashMap<String, String>,
    /// Per-account errors
    pub errors: HashMap<String, String>,
}

/// Tauri command: claim daily check-in for all saved accounts (in parallel).
#[tauri::command]
pub async fn claim_checkin_all(
    paths: State<'_, AppPaths>,
) -> AppResult<ClaimAllResult> {
    let state_data = state::read_state(&paths)?;
    let profiles_dir = paths.profiles_dir.clone();
    let app_data_dir = paths.app_data_dir.clone();

    log::info!("[checkin-claim] claim_checkin_all: {} accounts in state", state_data.accounts.len());

    let mut handles = Vec::new();

    for account in &state_data.accounts {
        if !account.saved {
            continue;
        }
        let id = account.id.clone();
        let profile_dir = profiles_dir.join(&id);
        let app_data_dir = app_data_dir.clone();

        if !profile_dir.exists() {
            continue;
        }

        let handle = tokio::spawn(async move {
            match quota::claim_checkin_for_profile(&profile_dir, &app_data_dir).await {
                Ok(result) => {
                    let status = result.result.clone().unwrap_or_else(|| "UNKNOWN".to_string());
                    log::info!("[checkin-claim] account {}: {}", id, status);
                    (id, Ok(result))
                }
                Err(e) => {
                    log::warn!("[checkin-claim] account {} failed: {}", id, e);
                    (id, Err(e.to_string()))
                }
            }
        });
        handles.push(handle);
    }

    let mut results: HashMap<String, String> = HashMap::new();
    let mut errors: HashMap<String, String> = HashMap::new();

    for handle in handles {
        if let Ok((id, result)) = handle.await {
            match result {
                Ok(claim) => {
                    let status = claim.result.unwrap_or_else(|| "UNKNOWN".to_string());
                    results.insert(id, status);
                }
                Err(e) => {
                    errors.insert(id, e);
                }
            }
        }
    }

    Ok(ClaimAllResult { results, errors })
}
