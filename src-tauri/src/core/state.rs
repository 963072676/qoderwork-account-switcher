use crate::core::paths::AppPaths;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::fs;

/// Root state persisted to state.json.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct State {
    pub accounts: Vec<Account>,
    #[serde(default)]
    pub active: Option<String>,
}

/// A single managed account.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Account {
    pub id: String,
    pub phone: String,
    pub label: String,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub saved: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            accounts: Vec::new(),
            active: None,
        }
    }
}

/// Read state from the state.json file inside profiles_dir.
/// Returns a default (empty) state if the file does not exist or is unreadable.
pub fn read_state(paths: &AppPaths) -> AppResult<State> {
    let state_file = paths.state_file();

    if !state_file.exists() {
        log::info!("No state file found at {:?}, returning default", state_file);
        return Ok(State::default());
    }

    let content = fs::read_to_string(&state_file).map_err(|e| {
        AppError::StateFile(format!("Failed to read state file {:?}: {}", state_file, e))
    })?;

    if content.trim().is_empty() {
        log::warn!("State file is empty, returning default");
        return Ok(State::default());
    }

    let state: State = serde_json::from_str(&content).map_err(|e| {
        AppError::StateFile(format!("Failed to parse state file {:?}: {}", state_file, e))
    })?;

    Ok(state)
}

/// Write state to the state.json file inside profiles_dir.
pub fn write_state(paths: &AppPaths, state: &State) -> AppResult<()> {
    let state_file = paths.state_file();

    // Ensure parent directory exists
    if let Some(parent) = state_file.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            AppError::StateFile(format!(
                "Failed to create directory {:?}: {}",
                parent, e
            ))
        })?;
    }

    let content = serde_json::to_string_pretty(state)?;
    fs::write(&state_file, content).map_err(|e| {
        AppError::StateFile(format!("Failed to write state file {:?}: {}", state_file, e))
    })?;

    Ok(())
}

/// Generate a unique account ID that does not collide with existing accounts.
/// Uses a short hex suffix derived from the current timestamp.
pub fn generate_unique_id(accounts: &[Account]) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut attempt = 0u32;
    loop {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        // Mix in attempt counter to avoid collision in tight loops
        let id = format!("acc_{:x}_{:x}", ts as u64, attempt);

        if !accounts.iter().any(|a| a.id == id) {
            return id;
        }

        attempt += 1;
        if attempt > 1000 {
            // Extremely unlikely — fall back to UUID-style
            return format!("acc_{}_{}", ts, attempt);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_unique_id_empty() {
        let accounts: Vec<Account> = vec![];
        let id = generate_unique_id(&accounts);
        assert!(id.starts_with("acc_"));
    }

    #[test]
    fn test_generate_unique_id_no_collision() {
        let accounts = vec![Account {
            id: "acc_test".to_string(),
            phone: "123".to_string(),
            label: "Test".to_string(),
            user_id: None,
            saved: false,
        }];
        let id = generate_unique_id(&accounts);
        assert_ne!(id, "acc_test");
    }

    #[test]
    fn test_state_serialization_roundtrip() {
        let state = State {
            accounts: vec![Account {
                id: "acc_1".to_string(),
                phone: "13800001111".to_string(),
                label: "Main".to_string(),
                user_id: Some("user-123".to_string()),
                saved: true,
            }],
            active: Some("acc_1".to_string()),
        };

        let json = serde_json::to_string(&state).unwrap();
        let parsed: State = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.accounts.len(), 1);
        assert_eq!(parsed.accounts[0].phone, "13800001111");
        assert_eq!(parsed.active, Some("acc_1".to_string()));
    }

    #[test]
    fn test_state_deserialization_missing_optional_fields() {
        let json = r#"{
            "accounts": [
                {
                    "id": "acc_1",
                    "phone": "13800001111",
                    "label": "Main"
                }
            ]
        }"#;

        let state: State = serde_json::from_str(json).unwrap();
        assert_eq!(state.accounts[0].user_id, None);
        assert!(!state.accounts[0].saved);
        assert_eq!(state.active, None);
    }
}
