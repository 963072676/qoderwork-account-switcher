use crate::core::paths::AppPaths;
use regex::Regex;
use std::fs;

/// Status file structure (partial — we care about avatar_url for userId extraction).
#[derive(serde::Deserialize, Debug)]
struct StatusFile {
    #[serde(default)]
    logged_in: Option<bool>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    avatar_url: Option<String>,
}

/// Read the `.status.json` file and extract the current user's userId.
///
/// The userId is the UUID embedded in the `avatar_url` field (pattern: `users/<uuid>/`).
/// This UUID matches the user_id stored in state.json accounts.
///
/// Returns `None` if the file does not exist, user is not logged in, or no UUID is found.
pub fn get_current_user_id(paths: &AppPaths) -> Option<String> {
    let status_path = &paths.status_file;

    if !status_path.exists() {
        log::warn!("[status] Status file not found at {:?}", status_path);
        return None;
    }

    let content = match fs::read_to_string(status_path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("[status] Failed to read status file {:?}: {}", status_path, e);
            return None;
        }
    };

    // Strip UTF-8 BOM if present
    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);

    let status: StatusFile = match serde_json::from_str(content) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("[status] Failed to parse status file: {}", e);
            return None;
        }
    };

    // Check if user is logged in
    if status.logged_in == Some(false) {
        log::info!("[status] User is not logged in (logged_in=false)");
        return None;
    }

    // Extract UUID from avatar_url: look for pattern users/<uuid>
    if let Some(ref avatar_url) = status.avatar_url {
        let re = match Regex::new(r"users/([0-9a-fA-F\-]{36})") {
            Ok(r) => r,
            Err(e) => {
                log::error!("[status] Failed to compile regex: {}", e);
                return None;
            }
        };

        if let Some(captures) = re.captures(avatar_url) {
            if let Some(user_id_match) = captures.get(1) {
                let user_id = user_id_match.as_str().to_lowercase();
                log::info!("[status] Detected current userId from avatar_url: {}", user_id);
                return Some(user_id);
            }
        }

        log::warn!("[status] No UUID pattern found in avatar_url: {}", avatar_url);
    } else {
        log::warn!("[status] avatar_url field is missing or null in status file");
    }

    log::info!("[status] Status file debug — logged_in: {:?}, username: {:?}, avatar_url: {:?}",
        status.logged_in, status.username, status.avatar_url);

    None
}

/// Diagnostic info for troubleshooting detection issues on other machines.
#[derive(serde::Serialize, Debug)]
pub struct StatusDebugInfo {
    pub status_file_exists: bool,
    pub status_file_path: String,
    pub logged_in: Option<bool>,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub detected_user_id: Option<String>,
    pub app_data_dir: String,
    pub app_data_dir_exists: bool,
    pub partitions_dir_exists: bool,
}

/// Get diagnostic information about the current status file and paths.
pub fn get_status_debug_info(paths: &AppPaths) -> StatusDebugInfo {
    let status_path = &paths.status_file;
    let status_file_exists = status_path.exists();

    let mut logged_in = None;
    let mut username = None;
    let mut avatar_url = None;

    if status_file_exists {
        if let Ok(content) = fs::read_to_string(status_path) {
            let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
            if let Ok(status) = serde_json::from_str::<StatusFile>(content) {
                logged_in = status.logged_in;
                username = status.username;
                avatar_url = status.avatar_url;
            }
        }
    }

    let detected_user_id = get_current_user_id(paths);

    StatusDebugInfo {
        status_file_exists,
        status_file_path: status_path.to_string_lossy().to_string(),
        logged_in,
        username,
        avatar_url,
        detected_user_id,
        app_data_dir: paths.app_data_dir.to_string_lossy().to_string(),
        app_data_dir_exists: paths.app_data_dir.exists(),
        partitions_dir_exists: paths.partitions_main.exists(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_user_id_from_avatar_url() {
        let re = Regex::new(r"users/([0-9a-fA-F\-]{36})").unwrap();
        let url = "https://cdn.example.com/users/550e8400-e29b-41d4-a716-446655440000/avatar.png";
        let captures = re.captures(url).unwrap();
        assert_eq!(
            captures.get(1).unwrap().as_str().to_lowercase(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn test_no_match_for_invalid_uuid() {
        let re = Regex::new(r"users/([0-9a-fA-F\-]{36})").unwrap();
        let url = "https://cdn.example.com/users/not-a-uuid/avatar.png";
        assert!(re.captures(url).is_none());
    }
}
