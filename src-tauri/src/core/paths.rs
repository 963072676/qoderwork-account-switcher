use crate::error::{AppError, AppResult};
use std::path::PathBuf;

/// Cross-platform application paths resolved at startup.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AppPaths {
    /// ~/.qoderworkcn/account-profiles — stores per-account session backups
    pub profiles_dir: PathBuf,
    /// ~/.qoderworkcn/.auth-cn — shared auth directory
    pub auth_dir: PathBuf,
    /// %APPDATA%/QoderWork CN | ~/Library/Application Support/QoderWork CN
    pub app_data_dir: PathBuf,
    /// app_data_dir/Partitions/main
    pub partitions_main: PathBuf,
    /// app_data_dir/rum-electron-store
    pub rum_store: PathBuf,
    /// app_data_dir/lockfile
    pub lockfile: PathBuf,
    /// app_data_dir/auth.dat
    pub auth_dat: PathBuf,
    /// app_data_dir/auth-v2.dat
    pub auth_v2_dat: PathBuf,
    /// ~/.qoderworkcn/.status.json
    pub status_file: PathBuf,
}

impl AppPaths {
    /// Resolve all application paths using the current platform conventions.
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Could not resolve home directory");
        let qoderwork_home = home.join(".qoderworkcn");

        let profiles_dir = qoderwork_home.join("account-profiles");
        let auth_dir = qoderwork_home.join(".auth-cn");
        let status_file = qoderwork_home.join(".status.json");

        let app_data_dir = Self::resolve_app_data_dir();
        let partitions_main = app_data_dir.join("Partitions").join("main");
        let rum_store = app_data_dir.join("rum-electron-store");
        let lockfile = app_data_dir.join("lockfile");
        let auth_dat = app_data_dir.join("auth.dat");
        let auth_v2_dat = app_data_dir.join("auth-v2.dat");

        Self {
            profiles_dir,
            auth_dir,
            app_data_dir,
            partitions_main,
            rum_store,
            lockfile,
            auth_dat,
            auth_v2_dat,
            status_file,
        }
    }

    /// Resolve the application data directory per platform.
    fn resolve_app_data_dir() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            // %APPDATA%/QoderWork CN
            if let Some(appdata) = dirs::data_dir() {
                appdata.join("QoderWork CN")
            } else {
                // Fallback to home-based path
                let home = dirs::home_dir().expect("Could not resolve home directory");
                home.join("AppData").join("Roaming").join("QoderWork CN")
            }
        }

        #[cfg(target_os = "macos")]
        {
            // ~/Library/Application Support/QoderWork CN
            if let Some(app_support) = dirs::data_dir() {
                app_support.join("QoderWork CN")
            } else {
                let home = dirs::home_dir().expect("Could not resolve home directory");
                home.join("Library")
                    .join("Application Support")
                    .join("QoderWork CN")
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(data) = dirs::data_dir() {
                data.join("QoderWork CN")
            } else {
                let home = dirs::home_dir().expect("Could not resolve home directory");
                home.join(".local").join("share").join("QoderWork CN")
            }
        }
    }

    /// Return the per-account profile directory for a given account id.
    pub fn profile_dir(&self, account_id: &str) -> PathBuf {
        self.profiles_dir.join(account_id)
    }

    /// Return the path to state.json inside profiles_dir.
    pub fn state_file(&self) -> PathBuf {
        self.profiles_dir.join("state.json")
    }
}

/// Search common installation locations for the QoderWork CN executable.
/// Returns the path to the executable if found.
pub fn find_app_exe() -> AppResult<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        return find_app_exe_windows();
    }

    #[cfg(target_os = "macos")]
    {
        return find_app_exe_macos();
    }

    #[cfg(target_os = "linux")]
    {
        Err(AppError::AppNotFound(
            "QoderWork CN executable not found on Linux".to_string(),
        ))
    }
}

#[cfg(target_os = "windows")]
fn find_app_exe_windows() -> AppResult<PathBuf> {
    use std::path::Path;

    let exe_name = "QoderWork CN.exe";
    let dir_name = "QoderWork CN";

    // 1. Try registry-based discovery via known uninstall keys
    if let Some(path) = find_exe_from_registry() {
        if path.exists() {
            return Ok(path);
        }
    }

    // 2. Check LOCALAPPDATA
    if let Some(local_app_data) = dirs::data_local_dir() {
        let candidate = local_app_data.join(dir_name).join(exe_name);
        if candidate.exists() {
            return Ok(candidate);
        }
        // Some installers place it one level deeper
        let candidate = local_app_data
            .join("Programs")
            .join(dir_name)
            .join(exe_name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    // 3. Check Program Files on C: and D: drives
    for drive in &["C:", "D:"] {
        for pf in &["Program Files", "Program Files (x86)"] {
            let candidate = Path::new(drive)
                .join(pf)
                .join(dir_name)
                .join(exe_name);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    // 4. Check common user-level locations
    if let Some(home) = dirs::home_dir() {
        let candidate = home
            .join("AppData")
            .join("Local")
            .join(dir_name)
            .join(exe_name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(AppError::AppNotFound(
        "QoderWork CN executable not found. Please set the path manually.".to_string(),
    ))
}

#[cfg(target_os = "windows")]
fn find_exe_from_registry() -> Option<PathBuf> {
    use std::path::PathBuf;

    // We read registry using std::process::Command to avoid winreg dependency
    let keys = [
        r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ];

    for key_root in &keys {
        let output = std::process::Command::new("reg")
            .args(["query", key_root, "/s", "/v", "DisplayName"])
            .output()
            .ok()?;

        if !output.status.success() {
            continue;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut current_key = String::new();

        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("HKEY_") {
                current_key = trimmed.to_string();
            }
            if trimmed.contains("QoderWork CN") && !current_key.is_empty() {
                // Now query InstallLocation for this key
                if let Some(install_location) = query_reg_value(&current_key, "InstallLocation") {
                    let exe_path =
                        PathBuf::from(&install_location).join("QoderWork CN.exe");
                    if exe_path.exists() {
                        return Some(exe_path);
                    }
                }
                // Try DisplayIcon as fallback
                if let Some(display_icon) = query_reg_value(&current_key, "DisplayIcon") {
                    let icon_path = PathBuf::from(&display_icon);
                    if icon_path.exists() {
                        return Some(icon_path);
                    }
                }
            }
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn query_reg_value(key: &str, value_name: &str) -> Option<String> {
    let output = std::process::Command::new("reg")
        .args(["query", key, "/v", value_name])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.contains(value_name) {
            // Format: "    InstallLocation    REG_SZ    C:\Path\To\App"
            let parts: Vec<&str> = trimmed.split("REG_SZ").collect();
            if parts.len() >= 2 {
                return Some(parts[1].trim().to_string());
            }
        }
    }

    None
}

#[cfg(target_os = "macos")]
fn find_app_exe_macos() -> AppResult<PathBuf> {
    use std::path::Path;

    let app_name = "QoderWork CN.app";
    let binary_rel = Path::new("Contents")
        .join("MacOS")
        .join("QoderWork CN");

    // 1. /Applications
    let candidate = Path::new("/Applications").join(app_name).join(&binary_rel);
    if candidate.exists() {
        return Ok(candidate);
    }

    // 2. ~/Applications
    if let Some(home) = dirs::home_dir() {
        let candidate = home.join("Applications").join(app_name).join(&binary_rel);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    // 3. Try mdfind as fallback
    if let Ok(output) = std::process::Command::new("mdfind")
        .args(["kMDItemCFBundleIdentifier == 'com.wanghaochen.qoderwork-cn'"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let app_path = Path::new(line.trim());
            if app_path.exists() {
                let binary = app_path.join(&binary_rel);
                if binary.exists() {
                    return Ok(binary);
                }
            }
        }
    }

    Err(AppError::AppNotFound(
        "QoderWork CN executable not found. Please set the path manually.".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths_creation() {
        let paths = AppPaths::new();
        assert!(paths.profiles_dir.to_str().is_some());
        assert!(paths.status_file.to_str().is_some());
        assert!(paths.partitions_main.to_str().is_some());
    }
}
