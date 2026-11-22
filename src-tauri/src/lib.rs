mod backup_crypt;
mod backup_secret;
mod commands;
mod db;
mod domain;
mod error;
mod events;
mod export;
mod fs;
mod share;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init());
    #[cfg(target_os = "android")]
    let builder = builder.plugin(tauri_plugin_android_fs::init());
    builder
        .setup(|app| {
            let state = state::AppState::init(app.handle())?;
            app.asset_protocol_scope()
                .allow_directory(&state.paths.assets_dir, true)?;
            // 递归通配符在 Unix 上默认跳过点开头的路径段（scope 的
            // requireLiteralLeadingDot），.thumbs/.export 必须单独放行。
            for name in [".thumbs", ".export"] {
                let dir = state.paths.assets_dir.join(name);
                std::fs::create_dir_all(&dir)?;
                app.asset_protocol_scope().allow_directory(&dir, true)?;
            }
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
            commands::appearance::font_import,
            commands::appearance::font_remove,
            commands::appearance::theme_bg_import,
            commands::appearance::theme_bg_remove,
            commands::app::data_info,
            commands::app::asset_paths,
            commands::app::app_open_data_dir,
            commands::media::media_pick_import,
            commands::media::media_paste_import,
            commands::media::media_import_paths,
            commands::media::asset_gc,
            commands::media::asset_clean_unused,
            commands::media::asset_export_views,
            commands::data::data_export_dir,
            commands::data::data_import_md,
            commands::data::data_backup,
            commands::data::data_restore,
            commands::data::backup_password_get,
            commands::data::backup_password_set,
            commands::image::image_export,
            #[cfg(target_os = "android")]
            commands::android_data::image_export_android,
            #[cfg(target_os = "android")]
            commands::android_data::images_export_android,
            #[cfg(target_os = "android")]
            commands::android_data::data_export_android,
            #[cfg(target_os = "android")]
            commands::android_data::data_import_android,
            #[cfg(target_os = "android")]
            commands::android_data::data_backup_android,
            #[cfg(target_os = "android")]
            commands::android_data::data_pick_restore_android,
            #[cfg(target_os = "android")]
            commands::android_data::data_restore_android,
            commands::lock::app_lock_status,
            commands::lock::app_lock_set,
            commands::lock::app_lock_clear,
            commands::lock::app_lock_unlock,
            commands::lock::app_lock_engage,
            commands::share::share_drain,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
