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
/// On Windows, uses `taskkill /F /T /IM` to forcefully terminate entire process trees,
/// which is essential for Electron apps that spawn many child/renderer processes.
/// After killing, waits up to 10 seconds verifying all processes have exited.
pub fn kill_app() -> AppResult<()> {
    // Phase 1: Send kill signals
    #[cfg(target_os = "windows")]
    {
        kill_app_windows();
    }

    #[cfg(not(target_os = "windows"))]
    {
        let mut sys = System::new_all();
        sys.refresh_all();
        let killed = kill_matching_processes(&sys);
        if killed > 0 {
            log::info!("[kill] First pass: killed {} processes", killed);
            thread::sleep(Duration::from_millis(3000));
        }
        sys.refresh_all();
        let killed2 = kill_matching_processes(&sys);
        if killed2 > 0 {
            log::info!("[kill] Second pass: killed {} remaining", killed2);
            thread::sleep(Duration::from_millis(2000));
        }
    }

    // Phase 2: Wait and verify all processes have exited
    wait_for_exit(10);

    Ok(())
}

/// Windows-specific kill using taskkill /F /T /IM for each known exe name.
/// /F = force, /T = kill entire process tree (critical for Electron apps).
#[cfg(target_os = "windows")]
fn kill_app_windows() {
    let exe_names = ["QoderWork CN.exe", "qoderclicn.exe", "qodercli.exe"];

    for exe_name in &exe_names {
        match Command::new("taskkill")
            .args(["/F", "/T", "/IM", exe_name])
            .output()
        {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let stderr = String::from_utf8_lossy(&o.stderr);
                if o.status.success() {
                    log::info!("[kill] taskkill {} OK: {}", exe_name, stdout.trim());
                } else {
                    // "没有找到进程" = not running, that's fine
                    log::info!("[kill] taskkill {}: {}", exe_name, stderr.trim());
                }
            }
            Err(e) => {
                log::warn!("[kill] taskkill {} error: {}", exe_name, e);
            }
        }
    }

    // Sysinfo fallback — catches any processes taskkill missed
    let mut sys = System::new_all();
    sys.refresh_all();
    let extra = kill_matching_processes(&sys);
    if extra > 0 {
        log::info!("[kill] sysinfo fallback killed {} additional processes", extra);
    }
}

/// Wait up to `max_seconds` for all QoderWork CN processes to exit.
/// Returns early if all processes have exited.
fn wait_for_exit(max_seconds: u64) {
    for i in 1..=max_seconds {
        thread::sleep(Duration::from_secs(1));
        if !is_app_running() {
            log::info!("[kill] All processes exited after {}s", i);
            return;
        }
        log::info!("[kill] Waiting for processes to exit... {}/{}s", i, max_seconds);
    }
    log::warn!("[kill] Timed out after {}s — some processes may still be running", max_seconds);
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
