use crate::error::{AppError, AppResult};
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;
use sysinfo::System;

/// Process name patterns to match when looking for QoderWork CN processes.
/// These match against the executable file name (not the full path).
const PROCESS_EXE_NAMES: &[&str] = &[
    "qoderwork cn.exe",
    "qoderclicn.exe",
    "qodercli.exe",
    "qoderclicn",
    "qodercli",
];

/// Kill all QoderWork CN related processes.
///
/// Uses sysinfo to find and kill processes in an aggressive retry loop:
/// keeps scanning and killing until no matching processes remain or timeout.
/// This handles Electron apps with many child/renderer processes that may
/// take time to fully exit.
pub fn kill_app() -> AppResult<()> {
    // Aggressive retry loop: keep killing until all processes are gone
    let max_rounds = 5;
    for round in 1..=max_rounds {
        let mut sys = System::new_all();
        sys.refresh_all();
        let killed = kill_matching_processes(&sys);

        if killed == 0 {
            if round == 1 {
                log::info!("[kill] No QoderWork CN processes found");
            } else {
                log::info!("[kill] All processes terminated after {} rounds", round - 1);
            }
            return Ok(());
        }

        log::info!("[kill] Round {}: killed {} processes", round, killed);

        // Wait for processes to actually exit before next round
        let wait_ms = if round <= 2 { 3000 } else { 2000 };
        thread::sleep(Duration::from_millis(wait_ms));
    }

    // Final check
    if is_app_running() {
        log::warn!("[kill] Some processes may still be running after {} rounds", max_rounds);
    } else {
        log::info!("[kill] All processes terminated after {} rounds", max_rounds);
    }

    Ok(())
}

/// Check if a process matches QoderWork CN by its exe file name.
fn is_qoderwork_process(exe_path: &str, proc_name: &str) -> bool {
    let name_lower = proc_name.to_lowercase();
    let exe_lower = exe_path.to_lowercase();

    // Check process name
    for pattern in PROCESS_EXE_NAMES {
        if name_lower == *pattern || exe_lower.contains(pattern) {
            return true;
        }
    }

    // Also check the exe file name (last component of path)
    if let Some(exe_name) = std::path::Path::new(exe_path)
        .file_name()
        .and_then(|n| n.to_str())
    {
        let exe_name_lower = exe_name.to_lowercase();
        for pattern in PROCESS_EXE_NAMES {
            if exe_name_lower == *pattern {
                return true;
            }
        }
    }

    false
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
        if exe_lower.contains("account-switcher") || exe_lower.contains("qw-switcher") {
            continue;
        }

        if is_qoderwork_process(&exe_str, &name) {
            log::info!(
                "Killing process: pid={}, name={}, exe={}",
                pid,
                name,
                exe_str
            );

            process.kill();
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
        // Plain spawn — works correctly for GUI apps from GUI subsystem binaries.
        Command::new(exe_path)
            .spawn()
            .map_err(|e| {
                AppError::Process(format!("启动失败 {:?}: {}", exe_path, e))
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
pub fn is_app_running() -> bool {
    let mut sys = System::new_all();
    sys.refresh_all();

    for (_pid, process) in sys.processes() {
        let name = process.name().to_string_lossy().to_string();
        let exe_str = process
            .exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        if is_qoderwork_process(&exe_str, &name) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_patterns_defined() {
        assert!(!PROCESS_EXE_NAMES.is_empty());
        assert!(PROCESS_EXE_NAMES.contains(&"qoderwork cn.exe"));
    }
}
