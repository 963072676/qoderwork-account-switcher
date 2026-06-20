use crate::core::paths::AppPaths;
use crate::error::{AppError, AppResult};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Subdirectories inside Partitions/main that must be saved/restored.
const PARTITIONS_SUBDIRS: &[&str] = &[
    "Local Storage",
    "Session Storage",
    "Network",
    "Shared Dictionary",
];

/// Save the current session data for the given account.
///
/// This backs up:
/// - auth.dat and auth-v2.dat
/// - lockfile
/// - 4 Partitions/main subdirectories (Local Storage, Session Storage, Network, Shared Dictionary)
/// - rum-electron-store
///
/// Existing backups are preserved with a .bak suffix before overwriting.
pub fn save_auth_data(paths: &AppPaths, account_id: &str) -> AppResult<()> {
    let profile_dir = paths.profile_dir(account_id);

    // Ensure profile directory exists
    fs::create_dir_all(&profile_dir).map_err(|e| {
        AppError::Session(format!(
            "Failed to create profile directory {:?}: {}",
            profile_dir, e
        ))
    })?;

    // Save individual files
    save_file_with_backup(&paths.auth_dat, &profile_dir)?;
    save_file_with_backup(&paths.auth_v2_dat, &profile_dir)?;
    save_file_with_backup(&paths.lockfile, &profile_dir)?;

    // Save .auth-cn token files (id and user — critical for CLI authentication)
    save_file_with_backup(&paths.auth_dir.join("id"), &profile_dir)?;
    save_file_with_backup(&paths.auth_dir.join("user"), &profile_dir)?;

    // Save Partitions subdirectories
    let partitions_backup_dir = profile_dir.join("Partitions").join("main");
    fs::create_dir_all(&partitions_backup_dir).map_err(|e| {
        AppError::Session(format!(
            "Failed to create partitions backup dir {:?}: {}",
            partitions_backup_dir, e
        ))
    })?;

    for subdir_name in PARTITIONS_SUBDIRS {
        let src = paths.partitions_main.join(subdir_name);
        let dst = partitions_backup_dir.join(subdir_name);

        if src.exists() && src.is_dir() {
            // Remove old backup if it exists
            if dst.exists() {
                fs::remove_dir_all(&dst).map_err(|e| {
                    AppError::Session(format!(
                        "Failed to remove old backup {:?}: {}",
                        dst, e
                    ))
                })?;
            }
            copy_dir_recursive(&src, &dst)?;
        }
    }

    // Save RUM store
    let rum_backup = profile_dir.join("rum-electron-store");
    if paths.rum_store.exists() {
        if rum_backup.exists() {
            if rum_backup.is_dir() {
                fs::remove_dir_all(&rum_backup).map_err(|e| {
                    AppError::Session(format!(
                        "Failed to remove old RUM backup {:?}: {}",
                        rum_backup, e
                    ))
                })?;
            } else {
                fs::remove_file(&rum_backup).map_err(|e| {
                    AppError::Session(format!(
                        "Failed to remove old RUM backup file {:?}: {}",
                        rum_backup, e
                    ))
                })?;
            }
        }

        if paths.rum_store.is_dir() {
            copy_dir_recursive(&paths.rum_store, &rum_backup)?;
        } else {
            fs::copy(&paths.rum_store, &rum_backup).map_err(|e| {
                AppError::Session(format!(
                    "Failed to copy RUM store {:?} -> {:?}: {}",
                    paths.rum_store, rum_backup, e
                ))
            })?;
        }
    }

    log::info!("Saved session data for account {}", account_id);
    Ok(())
}

/// Restore session data from the given account's profile directory.
///
/// This is the reverse of `save_auth_data`: copies saved files back
/// to the application data directory.
pub fn restore_auth_data(paths: &AppPaths, account_id: &str) -> AppResult<()> {
    let profile_dir = paths.profile_dir(account_id);

    if !profile_dir.exists() {
        return Err(AppError::Session(format!(
            "Profile directory does not exist for account {}: {:?}",
            account_id, profile_dir
        )));
    }

    // Restore individual files
    restore_file(&profile_dir.join("auth.dat"), &paths.auth_dat)?;
    restore_file(&profile_dir.join("auth-v2.dat"), &paths.auth_v2_dat)?;
    restore_file(&profile_dir.join("lockfile"), &paths.lockfile)?;

    // Restore .auth-cn token files (id and user — critical for CLI authentication)
    if !paths.auth_dir.exists() {
        fs::create_dir_all(&paths.auth_dir).map_err(|e| {
            AppError::Session(format!(
                "Failed to create auth dir {:?}: {}",
                paths.auth_dir, e
            ))
        })?;
    }
    restore_file(&profile_dir.join("id"), &paths.auth_dir.join("id"))?;
    restore_file(&profile_dir.join("user"), &paths.auth_dir.join("user"))?;

    // Restore Partitions subdirectories
    let saved_partitions = profile_dir.join("Partitions").join("main");
    if saved_partitions.exists() {
        // Ensure target partitions dir exists
        fs::create_dir_all(&paths.partitions_main).map_err(|e| {
            AppError::Session(format!(
                "Failed to create partitions dir {:?}: {}",
                paths.partitions_main, e
            ))
        })?;

        for subdir_name in PARTITIONS_SUBDIRS {
            let src = saved_partitions.join(subdir_name);
            let dst = paths.partitions_main.join(subdir_name);

            if src.exists() && src.is_dir() {
                // Clear target before restoring
                if dst.exists() {
                    fs::remove_dir_all(&dst).map_err(|e| {
                        AppError::Session(format!(
                            "Failed to clear partition dir {:?}: {}",
                            dst, e
                        ))
                    })?;
                }
                copy_dir_recursive(&src, &dst)?;
            }
        }
    }

    // Restore RUM store
    let saved_rum = profile_dir.join("rum-electron-store");
    if saved_rum.exists() {
        if paths.rum_store.exists() {
            if paths.rum_store.is_dir() {
                fs::remove_dir_all(&paths.rum_store).map_err(|e| {
                    AppError::Session(format!(
                        "Failed to clear RUM store {:?}: {}",
                        paths.rum_store, e
                    ))
                })?;
            } else {
                fs::remove_file(&paths.rum_store).map_err(|e| {
                    AppError::Session(format!(
                        "Failed to clear RUM store file {:?}: {}",
                        paths.rum_store, e
                    ))
                })?;
            }
        }

        if saved_rum.is_dir() {
            copy_dir_recursive(&saved_rum, &paths.rum_store)?;
        } else {
            fs::copy(&saved_rum, &paths.rum_store).map_err(|e| {
                AppError::Session(format!(
                    "Failed to restore RUM store {:?} -> {:?}: {}",
                    saved_rum, paths.rum_store, e
                ))
            })?;
        }
    }

    log::info!("Restored session data for account {}", account_id);
    Ok(())
}

/// Clear all session data from the application data directory.
///
/// Removes auth files and clears partition directories and RUM store.
pub fn clear_session(paths: &AppPaths) -> AppResult<()> {
    // Remove individual auth files
    remove_file_if_exists(&paths.auth_dat)?;
    remove_file_if_exists(&paths.auth_v2_dat)?;
    remove_file_if_exists(&paths.lockfile)?;

    // Clear Partitions subdirectories
    for subdir_name in PARTITIONS_SUBDIRS {
        let dir = paths.partitions_main.join(subdir_name);
        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(|e| {
                AppError::Session(format!("Failed to clear partition {:?}: {}", dir, e))
            })?;
        }
    }

    // Clear RUM store
    if paths.rum_store.exists() {
        if paths.rum_store.is_dir() {
            fs::remove_dir_all(&paths.rum_store).map_err(|e| {
                AppError::Session(format!("Failed to clear RUM store {:?}: {}", paths.rum_store, e))
            })?;
        } else {
            fs::remove_file(&paths.rum_store).map_err(|e| {
                AppError::Session(format!(
                    "Failed to clear RUM store file {:?}: {}",
                    paths.rum_store, e
                ))
            })?;
        }
    }

    log::info!("Cleared all session data");
    Ok(())
}

/// Recursively copy a directory and all its contents.
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> AppResult<()> {
    if !src.exists() {
        return Ok(());
    }

    fs::create_dir_all(dst).map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create dir {:?}: {}", dst, e),
        ))
    })?;

    for entry in WalkDir::new(src).min_depth(1) {
        let entry = entry.map_err(|e| {
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("WalkDir error: {}", e),
            ))
        })?;

        let relative = entry
            .path()
            .strip_prefix(src)
            .map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Strip prefix error: {}", e),
                ))
            })?;

        let target = dst.join(relative);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create dir {:?}: {}", target, e),
                ))
            })?;
        } else {
            // Ensure parent directory exists
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    AppError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to create parent dir {:?}: {}", parent, e),
                    ))
                })?;
            }
            fs::copy(entry.path(), &target).map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Failed to copy {:?} -> {:?}: {}",
                        entry.path(),
                        target,
                        e
                    ),
                ))
            })?;
        }
    }

    Ok(())
}

/// Clear all contents of a directory without removing the directory itself.
#[allow(dead_code)]
pub fn clear_dir_contents(dir: &Path) -> AppResult<()> {
    if !dir.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(dir).map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to read dir {:?}: {}", dir, e),
        ))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("ReadDir error: {}", e),
            ))
        })?;

        let path = entry.path();
        if path.is_dir() {
            fs::remove_dir_all(&path).map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to remove dir {:?}: {}", path, e),
                ))
            })?;
        } else {
            fs::remove_file(&path).map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to remove file {:?}: {}", path, e),
                ))
            })?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Save a single file to the profile directory, creating a .bak of any existing
/// file at the destination first.
fn save_file_with_backup(src: &Path, profile_dir: &Path) -> AppResult<()> {
    let file_name = match src.file_name() {
        Some(name) => name,
        None => return Ok(()), // Skip if no filename
    };

    if !src.exists() {
        log::info!("Source file {:?} does not exist, skipping", src);
        return Ok(());
    }

    let dst = profile_dir.join(file_name);

    // Backup existing file
    if dst.exists() {
        let bak = dst.with_extension("bak");
        fs::copy(&dst, &bak).map_err(|e| {
            AppError::Session(format!(
                "Failed to create backup {:?} -> {:?}: {}",
                dst, bak, e
            ))
        })?;
    }

    fs::copy(src, &dst).map_err(|e| {
        AppError::Session(format!(
            "Failed to copy {:?} -> {:?}: {}",
            src, dst, e
        ))
    })?;

    Ok(())
}

/// Restore a single file from the profile directory to the destination.
fn restore_file(src: &Path, dst: &Path) -> AppResult<()> {
    if !src.exists() {
        log::info!("Profile file {:?} does not exist, skipping", src);
        return Ok(());
    }

    // Ensure parent directory exists
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            AppError::Session(format!(
                "Failed to create parent dir {:?}: {}",
                parent, e
            ))
        })?;
    }

    fs::copy(src, dst).map_err(|e| {
        AppError::Session(format!(
            "Failed to restore {:?} -> {:?}: {}",
            src, dst, e
        ))
    })?;

    Ok(())
}

/// Remove a file if it exists, ignoring "not found" errors.
fn remove_file_if_exists(path: &Path) -> AppResult<()> {
    if path.exists() {
        fs::remove_file(path).map_err(|e| {
            AppError::Session(format!("Failed to remove {:?}: {}", path, e))
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_copy_dir_recursive() {
        let tmp = std::env::temp_dir().join("qoderwork_test_copy_recursive");
        let src = tmp.join("src");
        let dst = tmp.join("dst");

        // Clean up from previous runs
        let _ = fs::remove_dir_all(&tmp);

        // Create source structure
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("file1.txt"), "hello").unwrap();
        fs::write(src.join("sub").join("file2.txt"), "world").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();

        assert!(dst.join("file1.txt").exists());
        assert!(dst.join("sub").join("file2.txt").exists());
        assert_eq!(fs::read_to_string(dst.join("file1.txt")).unwrap(), "hello");
        assert_eq!(
            fs::read_to_string(dst.join("sub").join("file2.txt")).unwrap(),
            "world"
        );

        // Cleanup
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_clear_dir_contents() {
        let tmp = std::env::temp_dir().join("qoderwork_test_clear_contents");
        let _ = fs::remove_dir_all(&tmp);

        fs::create_dir_all(tmp.join("sub")).unwrap();
        fs::write(tmp.join("file.txt"), "data").unwrap();
        fs::write(tmp.join("sub").join("nested.txt"), "nested").unwrap();

        clear_dir_contents(&tmp).unwrap();

        assert!(tmp.exists()); // dir itself remains
        assert!(!tmp.join("file.txt").exists());
        assert!(!tmp.join("sub").exists());

        let _ = fs::remove_dir_all(&tmp);
    }
}
