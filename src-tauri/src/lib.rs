mod commands;
mod db;
mod domain;
mod error;
mod events;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = state::AppState::init(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::journal::journal_list,
            commands::journal::journal_get,
            commands::journal::journal_save,
            commands::journal::journal_delete,
            commands::journal::journal_restore,
            commands::journal::journal_purge,
            commands::journal::journal_empty_trash,
            commands::journal::journal_trash_list,
            commands::journal::journal_set_pinned,
            commands::journal::journal_set_sensitive,
            commands::journal::day_marks,
            commands::search::search_query,
            commands::tag::tag_list,
            commands::tag::tag_create,
            commands::tag::tag_rename,
            commands::tag::tag_delete,
            commands::tag::tag_merge,
            commands::settings::settings_get_all,
            commands::settings::settings_patch,
            commands::app::data_info,
            commands::app::app_open_data_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
