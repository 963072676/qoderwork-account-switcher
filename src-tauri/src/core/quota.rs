//! Quota/credit information retrieval for QoderWork CN accounts.
//!
//! Reads the encrypted auth-v2.dat file (Electron safeStorage / DPAPI on Windows),
//! extracts the access token (refreshing if needed), and calls the QoderWork CN
//! OpenAPI to fetch credit/quota usage details.

use crate::core::paths::AppPaths;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

#[cfg(target_os = "windows")]
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::Engine;

// Cosy auth crypto imports
use base64::engine::general_purpose::STANDARD as B64;
use cipher::block_padding::Pkcs7;
use cipher::KeyIvInit;
use md5::Digest as Md5Digest;
use rsa::pkcs8::DecodePublicKey;
use rsa::Pkcs1v15Encrypt;

/// API base URL for QoderWork CN OpenAPI.
const API_BASE: &str = "https://openapi.qoder.com.cn";

/// Token refresh endpoint (unauthenticated).
const REFRESH_URL: &str = "https://openapi.qoder.com.cn/api/v1/deviceToken/refresh";

/// Daily check-in status endpoint (Sash API, authenticated).
const CHECKIN_API_URL: &str = "https://openapi.qoder.com.cn/sash/api/v1/me/daily-check-in/status";

/// Daily check-in claim endpoint (Sash API, authenticated POST).
const CHECKIN_CLAIM_URL: &str = "https://openapi.qoder.com.cn/sash/api/v1/me/daily-check-in/claim";

/// Model activity endpoint for daily free model quotas (Algo API, authenticated).
const ACTIVITY_API_URL: &str = "https://gateway.qoder.com.cn/algo/api/v2/activity";

/// RSA 1024-bit public key for Cosy auth encryption.
const RSA_PUBLIC_KEY_PEM: &str = "\
-----BEGIN PUBLIC KEY-----\n\
MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDA8iMH5c02LilrsERw9t6Pv5Nc\n\
4k6Pz1EaDicBMpdpxKduSZu5OANqUq8er4GM95omAGIOPOh+Nx0spthYA2BqGz+l\n\
6HRkPJ7S236FZz73In/KVuLnwI8JJ2CbuJap8kvheCCZpmAWpb/cPx/3Vr/J6I17\n\
XcW+ML9FoCI6AOvOzwIDAQAB\n\
-----END PUBLIC KEY-----";

/// Electron safeStorage v10 prefix indicating AES-256-GCM encryption on Windows.
const V10_PREFIX: &[u8] = b"v10";

/// "DPAPI" prefix in Local State encrypted_key (5 bytes).
#[cfg(target_os = "windows")]
const DPAPI_HEADER: &[u8] = b"DPAPI";

/// AES-GCM nonce size (12 bytes).
#[cfg(target_os = "windows")]
const GCM_NONCE_SIZE: usize = 12;

/// AES-GCM authentication tag size (16 bytes).
#[cfg(target_os = "windows")]
const GCM_TAG_SIZE: usize = 16;

/// Cached AES master key (decrypted from Local State via DPAPI).
#[cfg(target_os = "windows")]
static AES_KEY_CACHE: OnceLock<Vec<u8>> = OnceLock::new();

/// Quota usage information returned to the frontend.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QuotaInfo {
    pub user_id: Option<String>,
    pub user_type: Option<String>,
    pub total_usage_percentage: Option<f64>,
    pub is_quota_exceeded: Option<bool>,
    /// Subscription/plan expiry timestamp in epoch milliseconds.
    pub expires_at: Option<f64>,
    pub user_quota: Option<QuotaDetail>,
    pub add_on_quota: Option<QuotaDetail>,
    pub org_resource_package: Option<OrgQuota>,
    /// Daily check-in status (fetched separately from Sash API).
    pub check_in: Option<CheckInInfo>,
    /// Daily free model activity (from Algo API, e.g. Qwen 3.7 Max free uses).
    pub daily_free_model: Option<ModelActivity>,
    /// User email from auth-v2.dat (not from the API).
    pub email: Option<String>,
    /// User display name from auth-v2.dat.
    pub user_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QuotaDetail {
    pub total: Option<f64>,
    pub used: Option<f64>,
    pub remaining: Option<f64>,
    pub percentage: Option<f64>,
    pub unit: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OrgQuota {
    pub used: Option<f64>,
    pub cap: Option<f64>,
    pub remaining: Option<f64>,
    pub percentage: Option<f64>,
    pub available: Option<bool>,
    pub unit: Option<String>,
}

/// Daily check-in status information from the Sash API.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CheckInInfo {
    /// "CLAIMED_TODAY" | "CLAIMABLE" | "DISABLED"
    pub status: Option<String>,
    pub reward_credits: Option<f64>,
    pub total_reward_credits: Option<f64>,
    pub current_streak_days: Option<i64>,
    pub total_claim_days: Option<i64>,
}

/// Result of a check-in claim attempt.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CheckInClaimResult {
    /// "CLAIMED" | "ALREADY_CLAIMED"
    pub result: Option<String>,
    pub reward_credits: Option<f64>,
    pub expires_at: Option<f64>,
}

/// Internal: check-in claim API response.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct CheckInClaimResponse {
    result: Option<String>,
    reward_credits: Option<f64>,
    expires_at: Option<f64>,
}

/// Internal: raw check-in API response (same fields, just for deserialization).
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct CheckInResponse {
    status: Option<String>,
    reward_credits: Option<f64>,
    total_reward_credits: Option<f64>,
    current_streak_days: Option<i64>,
    total_claim_days: Option<i64>,
    next_claim_at: Option<f64>,
    last_claimed_at: Option<f64>,
    reward_expires_at: Option<f64>,
}

/// A single model activity (daily free uses for a specific model).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModelActivity {
    pub model_name: Option<String>,
    pub tag: Option<String>,
    pub limit: Option<i64>,
    pub used: Option<i64>,
    pub remaining: Option<i64>,
    pub reset_at: Option<f64>,
    pub status_text: Option<String>,
}

/// Internal: activity API wrapper response.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct ActivityListResponse {
    code: Option<i64>,
    msg: Option<String>,
    data: Option<ActivityData>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct ActivityData {
    activities: Option<Vec<ActivityItem>>,
    query_at: Option<f64>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct ActivityItem {
    #[serde(rename = "type")]
    activity_type: Option<String>,
    activity_id: Option<String>,
    model_name: Option<String>,
    tag: Option<String>,
    tag_style: Option<String>,
    model_keys: Option<Vec<String>>,
    limit: Option<i64>,
    used: Option<i64>,
    remaining: Option<i64>,
    reset_at: Option<f64>,
    reset_strategy: Option<String>,
    server_timezone: Option<String>,
    description: Option<String>,
    status_text: Option<String>,
    discount: Option<f64>,
    eligible: Option<bool>,
    ineligible_reason: Option<String>,
    activity_end_at: Option<f64>,
    detail_url: Option<String>,
}

/// Internal: decrypted auth-v2.dat structure.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct AuthV2Data {
    token: String,
    refresh_token: String,
    expires_at: Option<String>,
    user: Option<AuthUser>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct AuthUser {
    email: Option<String>,
    name: Option<String>,
    id: Option<String>,
}

/// Internal: token refresh response.
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct RefreshResponse {
    device_token: Option<String>,
    token: Option<String>,
    refresh_token: Option<String>,
}

/// Fetch quota/credit usage for the currently active (live) account.
pub async fn fetch_quota_usage(paths: &AppPaths) -> AppResult<QuotaInfo> {
    fetch_quota_from_auth_file(&paths.auth_v2_dat, &paths.app_data_dir).await
}

/// Fetch quota/credit usage for a saved account profile.
///
/// Reads the auth-v2.dat from the account's profile directory, decrypts it,
/// and calls the quota API using that account's token.
pub async fn fetch_quota_for_profile(profile_dir: &Path, app_data_dir: &Path) -> AppResult<QuotaInfo> {
    let auth_path = profile_dir.join("auth-v2.dat");
    fetch_quota_from_auth_file(&auth_path, app_data_dir).await
}

/// Shared internal: read a specific auth-v2.dat file, get token, call API.
async fn fetch_quota_from_auth_file(auth_path: &Path, app_data_dir: &Path) -> AppResult<QuotaInfo> {
    let auth_data = read_auth_v2_from(auth_path, app_data_dir)?;

    let email = auth_data.user.as_ref().and_then(|u| u.email.clone());
    let user_name = auth_data.user.as_ref().and_then(|u| u.name.clone());

    let token = get_valid_token(&auth_data).await?;

    // Fetch quota, check-in status, and model activities in parallel
    let (quota, checkin, activity) = tokio::join!(
        call_quota_api(&token),
        call_checkin_api(&token),
        call_activity_api(&auth_data)
    );

    log::info!("[quota] quota API result: {:?}", quota.is_ok());
    log::info!("[quota] checkin API result: {:?}", checkin.is_ok());
    log::info!("[quota] activity API result: {:?}", activity.is_ok());

    let mut quota = quota?;
    quota.email = email;
    quota.user_name = user_name;
    // Attach check-in info (non-fatal if it failed)
    match &checkin {
        Ok(info) => {
            log::info!("[quota] checkin status: {:?}", info.status);
            quota.check_in = Some(info.clone());
        }
        Err(e) => {
            log::warn!("[quota] checkin API failed: {}", e);
            quota.check_in = None;
        }
    }
    // Attach daily free model activity (non-fatal if it failed)
    match activity {
        Ok(act) => {
            log::info!("[quota] daily free model: {:?}", act.model_name);
            quota.daily_free_model = Some(act);
        }
        Err(e) => {
            log::warn!("[quota] activity API failed: {}", e);
            quota.daily_free_model = None;
        }
    }

    Ok(quota)
}

/// Read and decrypt an auth-v2.dat file from a specific path.
fn read_auth_v2_from(auth_path: &Path, app_data_dir: &Path) -> AppResult<AuthV2Data> {
    if !auth_path.exists() {
        return Err(AppError::Api(
            "auth-v2.dat 不存在，请先在 QoderWork CN 中登录。".to_string(),
        ));
    }

    let raw = fs::read(auth_path).map_err(|e| {
        AppError::Api(format!("无法读取 auth-v2.dat: {}", e))
    })?;

    let json_bytes = decrypt_auth_data(&raw, app_data_dir)?;
    let json_str = String::from_utf8(json_bytes).map_err(|e| {
        AppError::Api(format!("auth-v2.dat 解密后不是有效 UTF-8: {}", e))
    })?;

    let auth_data: AuthV2Data = serde_json::from_str(&json_str).map_err(|e| {
        AppError::Api(format!("auth-v2.dat JSON 解析失败: {}", e))
    })?;

    Ok(auth_data)
}

/// Get the AES-256 master key from Local State (cached after first call).
///
/// The key is stored in `Local State` as `os_crypt.encrypted_key`, which is
/// a base64-encoded blob: "DPAPI" (5 bytes) + raw DPAPI ciphertext.
/// Decrypting via Windows DPAPI yields a 32-byte AES-256 key.
#[cfg(target_os = "windows")]
fn get_aes_master_key(app_data_dir: &Path) -> AppResult<&'static [u8]> {
    let key = AES_KEY_CACHE.get_or_init(|| {
        load_and_decrypt_aes_key(app_data_dir).unwrap_or_default()
    });
    if key.is_empty() {
        return Err(AppError::Api("AES 主钥加载失败".to_string()));
    }
    Ok(key.as_slice())
}

/// Load and decrypt the AES master key from Local State.
#[cfg(target_os = "windows")]
fn load_and_decrypt_aes_key(app_data_dir: &Path) -> AppResult<Vec<u8>> {
    let local_state_path = app_data_dir.join("Local State");
    if !local_state_path.exists() {
        return Err(AppError::Api("Local State 文件不存在".to_string()));
    }

    let content = fs::read_to_string(&local_state_path).map_err(|e| {
        AppError::Api(format!("无法读取 Local State: {}", e))
    })?;

    let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        AppError::Api(format!("Local State JSON 解析失败: {}", e))
    })?;

    let encrypted_key_b64 = json
        .get("os_crypt")
        .and_then(|v| v.get("encrypted_key"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Api("Local State 中缺少 os_crypt.encrypted_key".to_string()))?;

    let encrypted_key_bytes = base64::engine::general_purpose::STANDARD
        .decode(encrypted_key_b64)
        .map_err(|e| AppError::Api(format!("encrypted_key base64 解码失败: {}", e)))?;

    if !encrypted_key_bytes.starts_with(DPAPI_HEADER) {
        return Err(AppError::Api(
            "encrypted_key 不以 DPAPI 开头".to_string(),
        ));
    }

    let dpapi_blob = &encrypted_key_bytes[DPAPI_HEADER.len()..];
    let key = windows_dpapi::decrypt_data(dpapi_blob, windows_dpapi::Scope::User, None)
        .map_err(|e| AppError::Api(format!("AES 主钥 DPAPI 解密失败: {}", e)))?;

    log::info!("AES master key decrypted: {} bytes", key.len());
    Ok(key)
}

/// Decrypt auth data using AES-256-GCM with key from Local State.
///
/// Format: "v10" (3 bytes) + nonce (12 bytes) + ciphertext (variable) + tag (16 bytes)
#[cfg(target_os = "windows")]
fn decrypt_auth_data(raw: &[u8], app_data_dir: &Path) -> AppResult<Vec<u8>> {
    if !raw.starts_with(V10_PREFIX) {
        // Maybe it's a plaintext JSON fallback?
        if raw.first() == Some(&b'{') {
            return Ok(raw.to_vec());
        }
        return Err(AppError::Api(
            "auth-v2.dat 格式未知（不以 v10 开头）".to_string(),
        ));
    }

    let aes_key = get_aes_master_key(app_data_dir)?;
    if aes_key.len() != 32 {
        return Err(AppError::Api(format!(
            "AES 主钥长度错误: {} (期望 32)",
            aes_key.len()
        )));
    }

    // Layout after "v10": nonce (12) + ciphertext (N) + tag (16)
    let payload = &raw[V10_PREFIX.len()..];
    let min_len = GCM_NONCE_SIZE + GCM_TAG_SIZE;
    if payload.len() < min_len {
        return Err(AppError::Api(format!(
            "auth-v2.dat 数据太短: {} bytes (最少 {})",
            payload.len(),
            min_len
        )));
    }

    let nonce_bytes = &payload[..GCM_NONCE_SIZE];
    let ct_and_tag = &payload[GCM_NONCE_SIZE..];

    let cipher = Aes256Gcm::new_from_slice(aes_key)
        .map_err(|e| AppError::Api(format!("AES-256-GCM 初始化失败: {}", e)))?;

    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ct_and_tag)
        .map_err(|e| AppError::Api(format!("AES-256-GCM 解密失败: {}", e)))?;

    Ok(plaintext)
}

/// Stub for non-Windows platforms — not yet implemented.
#[cfg(not(target_os = "windows"))]
fn decrypt_auth_data(_raw: &[u8], _app_data_dir: &Path) -> AppResult<Vec<u8>> {
    Err(AppError::Api(
        "额度查询目前仅支持 Windows 平台".to_string(),
    ))
}

/// Get a valid access token, refreshing if the current one is near expiry.
async fn get_valid_token(auth_data: &AuthV2Data) -> AppResult<String> {
    // Check if token needs refresh (expires within 1 hour)
    let needs_refresh = match &auth_data.expires_at {
        Some(expires_at_str) => {
            match parse_iso8601_to_epoch_secs(expires_at_str) {
                Some(expires_epoch) => {
                    let now_secs = current_epoch_secs();
                    // Refresh if token expires within 1 hour
                    expires_epoch - now_secs < 3600
                }
                None => false, // Can't parse — try using as-is
            }
        }
        None => false,
    };

    if needs_refresh {
        log::info!("Access token near expiry, refreshing...");
        match refresh_token(&auth_data.refresh_token).await {
            Ok(new_token) => {
                log::info!("Token refreshed successfully");
                return Ok(new_token);
            }
            Err(e) => {
                log::warn!("Token refresh failed ({}), trying with current token", e);
            }
        }
    }

    Ok(auth_data.token.clone())
}

/// Refresh the access token using the refresh_token endpoint.
async fn refresh_token(refresh_token: &str) -> AppResult<String> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "refresh_token": refresh_token,
    });

    let resp = client
        .post(REFRESH_URL)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Api(format!("Token 刷新请求失败: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Api(format!(
            "Token 刷新失败 (HTTP {}): {}",
            status, body
        )));
    }

    let refresh_resp: RefreshResponse = resp.json().await.map_err(|e| {
        AppError::Api(format!("Token 刷新响应解析失败: {}", e))
    })?;

    refresh_resp
        .device_token
        .or(refresh_resp.token)
        .ok_or_else(|| AppError::Api("Token 刷新响应中缺少 access token".to_string()))
}

/// Call the QoderWork CN quota usage API.
async fn call_quota_api(token: &str) -> AppResult<QuotaInfo> {
    let client = reqwest::Client::new();

    let url = format!("{}/api/v2/quota/usage", API_BASE);

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .header("User-Agent", "qoderwork-account-switcher/1.0")
        .send()
        .await
        .map_err(|e| AppError::Api(format!("额度查询请求失败: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Api(format!(
            "额度查询失败 (HTTP {}): {}",
            status, body
        )));
    }

    let body = resp.text().await.map_err(|e| {
        AppError::Api(format!("额度响应读取失败: {}", e))
    })?;
    log::info!("[quota] raw quota API response (first 500 chars): {}", &body[..body.len().min(500)]);

    let quota: QuotaInfo = serde_json::from_str(&body).map_err(|e| {
        AppError::Api(format!("额度响应解析失败: {} | body: {}", e, &body[..body.len().min(200)]))
    })?;

    Ok(quota)
}

/// Call the Sash daily-check-in status API.
async fn call_checkin_api(token: &str) -> AppResult<CheckInInfo> {
    let client = reqwest::Client::new();

    let resp = client
        .get(CHECKIN_API_URL)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .header("User-Agent", "QoderWork")
        .send()
        .await
        .map_err(|e| AppError::Api(format!("签到状态请求失败: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Api(format!(
            "签到状态查询失败 (HTTP {}): {}",
            status, body
        )));
    }

    let body = resp.text().await.map_err(|e| {
        AppError::Api(format!("签到响应读取失败: {}", e))
    })?;
    log::info!("[quota] raw checkin API response: {}", &body[..body.len().min(500)]);

    let checkin_resp: CheckInResponse = serde_json::from_str(&body).map_err(|e| {
        AppError::Api(format!("签到响应解析失败: {} | body: {}", e, &body[..body.len().min(200)]))
    })?;

    Ok(CheckInInfo {
        status: checkin_resp.status,
        reward_credits: checkin_resp.reward_credits,
        total_reward_credits: checkin_resp.total_reward_credits,
        current_streak_days: checkin_resp.current_streak_days,
        total_claim_days: checkin_resp.total_claim_days,
    })
}

/// --- Cosy Auth Helpers ---

/// AES-128-CBC encrypt with key used as both key and IV, PKCS7 padding, base64 output.
fn cosy_aes_encrypt(plaintext: &str, key: &str) -> AppResult<String> {
    use cipher::BlockEncryptMut;
    let key_bytes = key.as_bytes();
    let iv = &key_bytes[..16];
    type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
    let enc = Aes128CbcEnc::new_from_slices(key_bytes, iv)
        .map_err(|e| AppError::Api(format!("AES-128-CBC init failed: {}", e)))?;
    let pt = plaintext.as_bytes();
    // Buffer: plaintext length + one full block (16 bytes) for padding
    let block_size = 16;
    let buf_len = pt.len() + block_size;
    let mut buf = vec![0u8; buf_len];
    buf[..pt.len()].copy_from_slice(pt);
    let ct = enc
        .encrypt_padded_mut::<Pkcs7>(&mut buf, pt.len())
        .map_err(|e| AppError::Api(format!("AES-128-CBC encrypt failed: {}", e)))?;
    Ok(B64.encode(ct))
}

/// RSA PKCS1 encrypt with the hardcoded public key, base64 output.
fn cosy_rsa_encrypt(data: &[u8]) -> AppResult<String> {
    let pub_key = rsa::RsaPublicKey::from_public_key_pem(RSA_PUBLIC_KEY_PEM)
        .map_err(|e| AppError::Api(format!("RSA public key parse failed: {}", e)))?;
    let mut rng = rsa::rand_core::OsRng;
    let encrypted = pub_key
        .encrypt(&mut rng, Pkcs1v15Encrypt, data)
        .map_err(|e| AppError::Api(format!("RSA encrypt failed: {}", e)))?;
    Ok(B64.encode(&encrypted))
}

/// MD5 hash as lowercase hex string.
fn cosy_md5(input: &str) -> String {
    let mut hasher = md5::Md5::new();
    md5::Digest::update(&mut hasher, input.as_bytes());
    format!("{:x}", md5::Digest::finalize(hasher))
}

/// Generate UUID v4 without dashes (32 hex chars).
fn cosy_uuid() -> String {
    uuid::Uuid::new_v4().to_string().replace('-', "")
}

/// Cosy auth headers for the Algo/activity API.
struct CosyHeaders {
    request_id: String,
    authorization: String,
    cosy_user: String,
    cosy_key: String,
    cosy_date: String,
}

/// Build Cosy authentication headers from auth data.
fn build_cosy_headers(auth_data: &AuthV2Data) -> AppResult<CosyHeaders> {
    let user = auth_data
        .user
        .as_ref()
        .ok_or_else(|| AppError::Api("auth-v2.dat 中缺少 user 信息".to_string()))?;

    let uid = user
        .id
        .as_deref()
        .ok_or_else(|| AppError::Api("auth-v2.dat 中缺少 user.id".to_string()))?;
    let name = user.name.as_deref().unwrap_or("");
    let email = user.email.as_deref().unwrap_or("");

    // 1. encryptUserInfo
    let aes_key = cosy_uuid()[..16].to_string();
    let user_info = serde_json::json!({
        "uid": uid,
        "aid": "",
        "name": name,
        "email": email,
        "security_oauth_token": auth_data.token
    });
    let encrypted_info = cosy_aes_encrypt(&user_info.to_string(), &aes_key)?;
    let rsa_encrypted_key = cosy_rsa_encrypt(aes_key.as_bytes())?;

    // 2. generateAuthToken
    let timestamp = current_epoch_secs().to_string();
    let request_id = cosy_uuid();
    let payload = serde_json::json!({
        "version": "v1",
        "requestId": request_id,
        "info": encrypted_info,
        "cosyVersion": "1.0.0",
        "ideVersion": "1.0.0"
    });
    let payload_b64 = B64.encode(payload.to_string().as_bytes());

    // URL path: strip "/algo" prefix
    let path = ACTIVITY_API_URL
        .split("qoder.com.cn")
        .nth(1)
        .unwrap_or("/algo/api/v2/activity");
    let path = if path.starts_with("/algo") {
        &path[5..]
    } else {
        path
    };

    // Signature: "{payload_b64}\n{rsa_key}\n{timestamp}\n{body}\n{path}"
    let body = "";
    let sig_input = format!(
        "{}\n{}\n{}\n{}\n{}",
        payload_b64, rsa_encrypted_key, timestamp, body, path
    );
    let signature = cosy_md5(&sig_input);
    let auth_token = format!("Bearer COSY.{}.{}", payload_b64, signature);

    Ok(CosyHeaders {
        request_id,
        authorization: auth_token,
        cosy_user: uid.to_string(),
        cosy_key: rsa_encrypted_key,
        cosy_date: timestamp,
    })
}

/// Call the Algo activity API to get daily free model quota (e.g. Qwen 3.7 Max).
/// Uses Cosy authentication (AES-128-CBC + RSA + MD5 signature).
async fn call_activity_api(auth_data: &AuthV2Data) -> AppResult<ModelActivity> {
    let cosy = build_cosy_headers(auth_data)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| AppError::Api(format!("Activity HTTP client 创建失败: {}", e)))?;

    let resp = client
        .get(ACTIVITY_API_URL)
        .header("X-Request-Id", &cosy.request_id)
        .header("X-IDE-Platform", "qoder_work")
        .header("X-Version", "1.0.0")
        .header("X-Machine-OS", "win32")
        .header("Cosy-User", &cosy.cosy_user)
        .header("Cosy-Key", &cosy.cosy_key)
        .header("Cosy-Date", &cosy.cosy_date)
        .header("Authorization", &cosy.authorization)
        .header("Accept", "application/json")
        .header("User-Agent", "QoderWork")
        .send()
        .await
        .map_err(|e| AppError::Api(format!("活动额度请求失败: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Api(format!(
            "活动额度查询失败 (HTTP {}): {}",
            status, body
        )));
    }

    let body = resp.text().await.map_err(|e| {
        AppError::Api(format!("活动额度响应读取失败: {}", e))
    })?;
    log::info!("[quota] raw activity API response: {}", &body[..body.len().min(500)]);

    let activity_resp: ActivityListResponse = serde_json::from_str(&body).map_err(|e| {
        AppError::Api(format!("活动额度响应解析失败: {}", e))
    })?;

    let activities = activity_resp
        .data
        .and_then(|d| d.activities)
        .unwrap_or_default();

    if activities.is_empty() {
        return Err(AppError::Api("活动额度接口返回空列表".to_string()));
    }

    // Find the FREE-tagged activity (daily free model, e.g. Qwen 3.7 Max)
    let free_activity = activities
        .iter()
        .find(|a| a.tag.as_deref() == Some("FREE"))
        .or_else(|| activities.first());

    match free_activity {
        Some(a) => Ok(ModelActivity {
            model_name: a.model_name.clone(),
            tag: a.tag.clone(),
            limit: a.limit,
            used: a.used,
            remaining: a.remaining,
            reset_at: a.reset_at,
            status_text: a.status_text.clone(),
        }),
        None => Err(AppError::Api("未找到免费模型活动".to_string())),
    }
}

/// Claim daily check-in for a saved account profile.
///
/// Reads the auth-v2.dat from the account's profile directory, decrypts it,
/// and calls the check-in claim API using that account's token.
pub async fn claim_checkin_for_profile(profile_dir: &Path, app_data_dir: &Path) -> AppResult<CheckInClaimResult> {
    let auth_path = profile_dir.join("auth-v2.dat");
    let auth_data = read_auth_v2_from(&auth_path, app_data_dir)?;
    let token = get_valid_token(&auth_data).await?;
    call_checkin_claim(&token).await
}

/// Call the Sash daily-check-in claim API (POST).
async fn call_checkin_claim(token: &str) -> AppResult<CheckInClaimResult> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Api(format!("签到 claim HTTP client 创建失败: {}", e)))?;

    let resp = client
        .post(CHECKIN_CLAIM_URL)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("User-Agent", "QoderWork")
        .body("{}")
        .send()
        .await
        .map_err(|e| AppError::Api(format!("签到领取请求失败: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Api(format!(
            "签到领取失败 (HTTP {}): {}",
            status, body
        )));
    }

    let body = resp.text().await.map_err(|e| {
        AppError::Api(format!("签到领取响应读取失败: {}", e))
    })?;
    log::info!("[checkin-claim] raw response: {}", &body[..body.len().min(500)]);

    let claim_resp: CheckInClaimResponse = serde_json::from_str(&body).map_err(|e| {
        AppError::Api(format!("签到领取响应解析失败: {} | body: {}", e, &body[..body.len().min(200)]))
    })?;

    Ok(CheckInClaimResult {
        result: claim_resp.result,
        reward_credits: claim_resp.reward_credits,
        expires_at: claim_resp.expires_at,
    })
}

/// Parse a simplified ISO 8601 datetime string to Unix epoch seconds.
/// Supports formats like "2025-06-21T12:00:00.000Z" and "2025-06-21T12:00:00Z".
fn parse_iso8601_to_epoch_secs(s: &str) -> Option<i64> {
    // Parse: YYYY-MM-DDTHH:MM:SS[.fraction][Z|+HH:MM]
    let s = s.trim();
    if s.len() < 19 {
        return None;
    }

    let year: i64 = s[0..4].parse().ok()?;
    let month: i64 = s[5..7].parse().ok()?;
    let day: i64 = s[8..10].parse().ok()?;
    let hour: i64 = s[11..13].parse().ok()?;
    let min: i64 = s[14..16].parse().ok()?;
    let sec: i64 = s[17..19].parse().ok()?;

    // Convert to days since epoch (simplified algorithm)
    // Based on http://howardhinnant.github.io/date_algorithms.html
    let y = if month <= 2 { year - 1 } else { year };
    let m = if month <= 2 { month + 9 } else { month - 3 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64; // year of era [0, 399]
    let doy = (153 * m as u64 + 2) / 5 + day as u64 - 1; // day of year [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // day of era [0, 146096]
    let days_since_epoch = era as i64 * 146097 + doe as i64 - 719468;

    let total_secs = days_since_epoch * 86400 + hour * 3600 + min * 60 + sec;

    Some(total_secs)
}

/// Get current time as Unix epoch seconds.
fn current_epoch_secs() -> i64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iso8601() {
        // 2025-01-01T00:00:00Z = 1735689600
        let ts = parse_iso8601_to_epoch_secs("2025-01-01T00:00:00Z");
        assert_eq!(ts, Some(1735689600));

        // 1970-01-01T00:00:00Z = 0
        let ts = parse_iso8601_to_epoch_secs("1970-01-01T00:00:00Z");
        assert_eq!(ts, Some(0));
    }

    #[test]
    fn test_parse_iso8601_with_millis() {
        let ts = parse_iso8601_to_epoch_secs("2026-06-21T12:30:45.000Z");
        assert!(ts.is_some());
        let expected = parse_iso8601_to_epoch_secs("2026-06-21T12:30:45Z");
        assert_eq!(ts, expected);
    }
}
