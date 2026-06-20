use crate::error::{AppError, AppResult};
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;
use sysinfo::System;

/// Process name patterns to match when looking for QoderWork CN processes.
const PROCESS_PATTERNS: &[&str] = &["QoderWork", "qoderclicn", "qodercli"];

/// Kill all QoderWork CN related processes using sysinfo.
///
/// Uses a two-pass approach:
/// 1. First pass sends the kill signal to all matching processes.
/// 2. Wait briefly, then do a second pass to catch any that survived.
pub fn kill_app() -> AppResult<()> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let killed_first_pass = kill_matching_processes(&sys);

    if killed_first_pass > 0 {
        log::info!(
            "First pass: sent kill signal to {} processes",
            killed_first_pass
        );
        // Wait for processes to terminate (PS script waits 4s)
        thread::sleep(Duration::from_millis(4000));
    }

    // Second pass — refresh and kill any survivors
    sys.refresh_all();
    let killed_second_pass = kill_matching_processes(&sys);

    if killed_second_pass > 0 {
        log::info!(
            "Second pass: sent kill signal to {} remaining processes",
            killed_second_pass
        );
        thread::sleep(Duration::from_millis(2000));
    }

    let total_killed = killed_first_pass + killed_second_pass;
    if total_killed == 0 {
        log::info!("No QoderWork CN processes found to kill");
    } else {
        log::info!("Killed {} total processes", total_killed);
    }

    Ok(())
}

/// Find and kill all processes matching known QoderWork patterns.
/// Excludes the switcher's own process to avoid killing ourselves.
/// Returns the number of processes that were sent a kill signal.
fn kill_matching_processes(sys: &System) -> usize {
    let mut count = 0;

    for (pid, process) in sys.processes() {
        let name = process.name().to_string_lossy().to_string();
        let exe_str = process
            .exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        // Skip the switcher's own process
        let exe_lower = exe_str.to_lowercase();
        if exe_lower.contains("account-switcher") || exe_lower.contains("switcher") {
            continue;
        }

        let name_lower = name.to_lowercase();

        let mut matched = false;
        for pattern in PROCESS_PATTERNS {
            let pattern_lower = pattern.to_lowercase();
            if name_lower.contains(&pattern_lower) || exe_lower.contains(&pattern_lower) {
                matched = true;
                break;
            }
        }

        if matched {
            log::info!(
                "Killing process: pid={}, name={}, exe={}",
                pid,
                name,
                exe_str
            );

            // Use taskkill on Windows for more reliable termination
            #[cfg(target_os = "windows")]
            {
                let pid_u32 = pid.as_u32();
                let result = Command::new("taskkill")
                    .args(["/F", "/PID", &pid_u32.to_string()])
                    .output();
                match result {
                    Ok(output) if output.status.success() => {
                        log::info!("taskkill /F /PID {} succeeded", pid_u32);
                    }
                    _ => {
                        log::warn!("taskkill failed for PID {}, falling back to process.kill()", pid_u32);
                        process.kill();
                    }
                }
            }

            #[cfg(not(target_os = "windows"))]
            {
                process.kill();
            }

            count += 1;
        }
    }

    count
}

/// Launch the QoderWork CN application as a detached process.
///
/// The spawned process is detached from this application so it continues
/// running independently after launch.
pub fn launch_app(exe_path: &Path) -> AppResult<()> {
    if !exe_path.exists() {
        return Err(AppError::AppNotFound(format!(
            "Executable not found at {:?}",
            exe_path
        )));
    }

    log::info!("Launching app from {:?}", exe_path);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const DETACHED_PROCESS: u32 = 0x0000_0008;

        Command::new(exe_path)
            .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS)
            .spawn()
            .map_err(|e| {
                AppError::Process(format!("Failed to launch {:?}: {}", exe_path, e))
            })?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-a")
            .arg(exe_path)
            .spawn()
            .map_err(|e| {
                AppError::Process(format!("Failed to launch {:?}: {}", exe_path, e))
            })?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new(exe_path)
            .spawn()
            .map_err(|e| {
                AppError::Process(format!("Failed to launch {:?}: {}", exe_path, e))
            })?;
    }

    Ok(())
}

/// Check if any QoderWork CN processes are currently running.
#[allow(dead_code)]
pub fn is_app_running() -> bool {
    let mut sys = System::new_all();
    sys.refresh_all();

    for (_pid, process) in sys.processes() {
        let name = process.name().to_string_lossy().to_lowercase();
        let exe_str = process
            .exe()
            .map(|p| p.to_string_lossy().to_lowercase().to_string())
            .unwrap_or_default();

        for pattern in PROCESS_PATTERNS {
            let pattern_lower = pattern.to_lowercase();
            if name.contains(&pattern_lower) || exe_str.contains(&pattern_lower) {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_patterns_defined() {
        assert!(!PROCESS_PATTERNS.is_empty());
        assert!(PROCESS_PATTERNS.contains(&"QoderWork"));
    }
}
