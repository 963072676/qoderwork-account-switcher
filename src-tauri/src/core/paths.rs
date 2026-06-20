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

    // 1. Try registry-based discovery via known uninstall keys (native, no reg.exe)
    if let Some(path) = find_exe_from_registry() {
        if path.exists() {
            log::info!("Found exe via registry: {:?}", path);
            return Ok(path);
        }
    }

    // 2. Check LOCALAPPDATA
    if let Some(local_app_data) = dirs::data_local_dir() {
        let candidate = local_app_data.join(dir_name).join(exe_name);
        if candidate.exists() {
            return Ok(candidate);
        }
        let candidate = local_app_data
            .join("Programs")
            .join(dir_name)
            .join(exe_name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    // 3. Check Program Files on C: and D: drives
    //    Also check nested layout: "QoderWork CN/QoderWork CN/QoderWork CN.exe"
    for drive in &["C:", "D:", "E:"] {
        for pf in &["Program Files", "Program Files (x86)"] {
            let base = Path::new(drive).join(pf).join(dir_name);

            // Flat: Program Files/QoderWork CN/QoderWork CN.exe
            let candidate = base.join(exe_name);
            if candidate.exists() {
                return Ok(candidate);
            }
            // Nested: Program Files/QoderWork CN/QoderWork CN/QoderWork CN.exe
            let candidate = base.join(dir_name).join(exe_name);
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
        "QoderWork CN executable not found. Please set the path manually in Settings.".to_string(),
    ))
}

#[cfg(target_os = "windows")]
fn find_exe_from_registry() -> Option<PathBuf> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;

    let uninstall_roots: &[(fn() -> RegKey, &str)] = &[
        (|| RegKey::predef(HKEY_LOCAL_MACHINE), r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
        (|| RegKey::predef(HKEY_CURRENT_USER), r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
        (|| RegKey::predef(HKEY_LOCAL_MACHINE), r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"),
    ];

    for (make_root, path) in uninstall_roots {
        let base_key = match make_root().open_subkey_with_flags(path, KEY_READ) {
            Ok(k) => k,
            Err(_) => continue,
        };

        for subkey_name in base_key.enum_keys().flatten() {
            let subkey = match base_key.open_subkey_with_flags(&subkey_name, KEY_READ) {
                Ok(k) => k,
                Err(_) => continue,
            };

            let display_name: String = match subkey.get_value("DisplayName") {
                Ok(v) => v,
                Err(_) => continue,
            };

            if !display_name.contains("QoderWork CN") {
                continue;
            }

            // Try InstallLocation
            if let Ok(install_loc) = subkey.get_value::<String, _>("InstallLocation") {
                let exe_path = PathBuf::from(&install_loc).join("QoderWork CN.exe");
                if exe_path.exists() {
                    return Some(exe_path);
                }
                // Nested layout
                let exe_path = PathBuf::from(&install_loc).join("QoderWork CN").join("QoderWork CN.exe");
                if exe_path.exists() {
                    return Some(exe_path);
                }
            }

            // Try DisplayIcon — but only if it actually points to an .exe file
            // (some installers set DisplayIcon to a .ico file, which is not an executable)
            if let Ok(icon_path) = subkey.get_value::<String, _>("DisplayIcon") {
                let icon_path = PathBuf::from(&icon_path);
                if icon_path.exists() {
                    if let Some(ext) = icon_path.extension() {
                        if ext.eq_ignore_ascii_case("exe") {
                            return Some(icon_path);
                        }
                    }
                }
            }

            // Try UninstallString — parse the uninstaller path and look for
            // the main exe in the same directory
            if let Ok(uninstall_str) = subkey.get_value::<String, _>("UninstallString") {
                // UninstallString format: "path\to\Uninstall.exe" /flags
                // Strip quotes and arguments to get the directory
                let trimmed = uninstall_str.trim().trim_matches('"');
                if let Some(uninstall_exe) = trimmed.split_whitespace().next() {
                    let uninstall_path = PathBuf::from(uninstall_exe.trim_matches('"'));
                    if let Some(dir) = uninstall_path.parent() {
                        // Flat layout
                        let exe_path = dir.join("QoderWork CN.exe");
                        if exe_path.exists() {
                            return Some(exe_path);
                        }
                        // Nested layout
                        let exe_path = dir.join("QoderWork CN").join("QoderWork CN.exe");
                        if exe_path.exists() {
                            return Some(exe_path);
                        }
                    }
                }
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
