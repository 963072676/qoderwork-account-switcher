use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};

const GITHUB_REPO: &str = "963072676/qoderwork-account-switcher";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub available: bool,
    pub version: String,
    pub release_notes: String,
    pub download_url: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Compare two semver version strings (e.g. "1.0.1" vs "1.0.2").
/// Returns true if `latest` is newer than `current`.
fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> (u32, u32, u32) {
        let parts: Vec<&str> = v.trim_start_matches('v').split('.').collect();
        let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
        (major, minor, patch)
    };

    let latest = parse(latest);
    let current = parse(current);
    latest > current
}

/// Check the GitHub Releases API for a newer version of the application.
/// Returns update information including whether an update is available,
/// the latest version, release notes, and download URL.
#[tauri::command]
pub async fn check_update() -> AppResult<UpdateInfo> {
    let current_version = env!("CARGO_PKG_VERSION");

    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let client = reqwest::Client::builder()
        .user_agent("qoderwork-account-switcher")
        .build()
        .map_err(|e| AppError::Api(format!("Failed to create HTTP client: {}", e)))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| AppError::Api(format!("Failed to check for updates: {}", e)))?;

    if !response.status().is_success() {
        return Err(AppError::Api(format!(
            "GitHub API returned status {}",
            response.status()
        )));
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| AppError::Api(format!("Failed to parse release info: {}", e)))?;

    let latest_version = release.tag_name.trim_start_matches('v');
    let available = is_newer(latest_version, current_version);

    // Find the NSIS setup.exe asset
    let download_url = release
        .assets
        .iter()
        .find(|a| a.name.contains("setup.exe"))
        .map(|a| a.browser_download_url.clone())
        .unwrap_or_else(|| {
            format!(
                "https://github.com/{}/releases/latest",
                GITHUB_REPO
            )
        });

    log::info!(
        "[update] Current: {}, Latest: {}, Available: {}",
        current_version,
        latest_version,
        available
    );

    Ok(UpdateInfo {
        available,
        version: latest_version.to_string(),
        release_notes: release.body.unwrap_or_default(),
        download_url,
    })
}

/// Open a URL in the system default browser.
#[tauri::command]
pub fn open_url(url: String) -> AppResult<()> {
    // Basic URL validation
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(AppError::Api("Invalid URL".to_string()));
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &url])
            .spawn()
            .map_err(|e| AppError::Io(e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| AppError::Io(e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| AppError::Io(e))?;
    }

    log::info!("[update] Opened URL: {}", url);
    Ok(())
}
