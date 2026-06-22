mod commands;
mod core;
mod error;

use core::paths::AppPaths;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let paths = AppPaths::new();
            // Ensure profiles directory exists
            if let Err(e) = std::fs::create_dir_all(&paths.profiles_dir) {
                log::error!("Failed to create profiles directory: {}", e);
            }
            app.manage(paths);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::accounts::list_accounts,
            commands::accounts::add_account,
            commands::accounts::delete_account,
            commands::switch::switch_account,
            commands::switch::save_current_account,
            commands::detect::detect_current_account,
            commands::detect::detect_current_user_id,
            commands::detect::get_debug_info,
            commands::detect::get_exe_path,
            commands::detect::set_exe_path,
            commands::detect::auto_detect_exe,
            commands::quota_cmd::get_quota_usage,
            commands::quota_cmd::get_account_quota,
            commands::quota_cmd::refresh_all_quotas,
            commands::quota_cmd::claim_checkin_all,
            commands::update::check_update,
            commands::update::open_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
