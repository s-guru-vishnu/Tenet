pub mod agent;
pub mod api;
pub mod cli;
pub mod metadata;
pub mod processor;
pub mod storage;
pub mod tools;
pub mod utils;
pub mod versioning;
pub mod watcher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|_app| {
            // Optional: Any setup logic here
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            api::watch_directory,
            api::get_history,
            api::restore_version,
            api::get_status,
            api::get_tracked_files,
            api::get_file_content,
            api::get_ignore_rules,
            api::save_ignore_rules,
            // AI Agent
            api::run_agent,
            api::diff_versions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
