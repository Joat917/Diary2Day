use serde::Serialize;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

use crate::{
    commands::run_blocking,
    error::AppError,
    state::{AppState, dir_size},
};

#[derive(Debug, Serialize)]
pub struct Counts {
    pub journals: i64,
    pub trash: i64,
    pub tags: i64,
    pub assets: i64,
    pub words: i64,
}

#[derive(Debug, Serialize)]
pub struct AssetPaths {
    pub assets_dir: String,
}

/// Not lock-gated: the frontend needs the path before unlock to build asset URLs.
#[tauri::command]
pub fn asset_paths(state: State<'_, AppState>) -> AssetPaths {
    AssetPaths {
        assets_dir: state.paths.assets_dir.to_string_lossy().into_owned(),
    }
}

#[derive(Debug, Serialize)]
pub struct DataInfo {
    pub db_path: Option<String>,
    pub data_dir: String,
    pub assets_dir: String,
    pub db_size: u64,
    pub assets_size: u64,
    pub counts: Counts,
    pub mobile: bool,
}

#[tauri::command]
pub async fn data_info(state: State<'_, AppState>) -> Result<DataInfo, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| {
        let counts = state.with_db(|conn| {
            let journals: i64 = conn.query_row(
                "SELECT COUNT(*) FROM journal WHERE deleted_at IS NULL",
                [],
                |row| row.get(0),
            )?;
            let trash: i64 = conn.query_row(
                "SELECT COUNT(*) FROM journal WHERE deleted_at IS NOT NULL",
                [],
                |row| row.get(0),
            )?;
            let tags: i64 = conn.query_row("SELECT COUNT(*) FROM tag", [], |row| row.get(0))?;
            let assets: i64 = conn.query_row("SELECT COUNT(*) FROM asset", [], |row| row.get(0))?;
            let words: i64 = conn.query_row(
                "SELECT coalesce(SUM(words), 0) FROM journal WHERE deleted_at IS NULL",
                [],
                |row| row.get(0),
            )?;
            Ok(Counts {
                journals,
                trash,
                tags,
                assets,
                words,
            })
        })?;

        let db_size = std::fs::metadata(&state.paths.db_path)
            .map(|meta| meta.len())
            .unwrap_or(0);
        let assets_size = dir_size(&state.paths.assets_dir);
        let mobile = cfg!(any(target_os = "android", target_os = "ios"));

        Ok(DataInfo {
            db_path: if mobile {
                None
            } else {
                Some(state.paths.db_path.to_string_lossy().into_owned())
            },
            data_dir: state.paths.data_dir.to_string_lossy().into_owned(),
            assets_dir: state.paths.assets_dir.to_string_lossy().into_owned(),
            db_size,
            assets_size,
            counts,
            mobile,
        })
    })
    .await
}

#[tauri::command]
pub async fn app_open_data_dir(state: State<'_, AppState>) -> Result<(), AppError> {
    let path = state.paths.data_dir.to_string_lossy().into_owned();
    state
        .app
        .opener()
        .open_path(path, None::<&str>)
        .map_err(|err| AppError::io(err.to_string()))
}
