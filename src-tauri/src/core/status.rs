use crate::core::paths::AppPaths;
use regex::Regex;
use std::fs;

/// Status file structure (partial — we only care about avatar_url for userId extraction).
#[derive(serde::Deserialize, Debug)]
struct StatusFile {
    #[serde(default)]
    avatar_url: Option<String>,
}

/// Read the `.status.json` file and extract the current user's userId.
///
/// The userId is embedded in the `avatar_url` field as a UUID segment:
/// e.g. `https://cdn.example.com/users/550e8400-e29b-41d4-a716-446655440000/avatar.png`
///
/// Returns `None` if the file does not exist, cannot be parsed, or no UUID is found.
pub fn get_current_user_id(paths: &AppPaths) -> Option<String> {
    let status_path = &paths.status_file;

    if !status_path.exists() {
        log::info!("Status file not found at {:?}", status_path);
        return None;
    }

    let content = match fs::read_to_string(status_path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to read status file {:?}: {}", status_path, e);
            return None;
        }
    };

    let status: StatusFile = match serde_json::from_str(&content) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Failed to parse status file: {}", e);
            return None;
        }
    };

    let avatar_url = status.avatar_url?;

    // Extract UUID from avatar_url: look for pattern users/<uuid>
    let re = match Regex::new(r"users/([0-9a-fA-F\-]{36})") {
        Ok(r) => r,
        Err(e) => {
            log::error!("Failed to compile regex: {}", e);
            return None;
        }
    };

    if let Some(captures) = re.captures(&avatar_url) {
        if let Some(user_id_match) = captures.get(1) {
            let user_id = user_id_match.as_str().to_lowercase();
            log::info!("Detected current userId: {}", user_id);
            return Some(user_id);
        }
    }

    log::info!("No userId found in avatar_url: {}", avatar_url);
    None
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
