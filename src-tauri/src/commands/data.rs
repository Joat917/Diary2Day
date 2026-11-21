use tauri::State;

use crate::{
    commands::run_blocking,
    error::AppError,
    export::{self, BackupReport, ExportReport, ImportReport},
    state::AppState,
};

#[tauri::command]
pub async fn data_export_dir(
    state: State<'_, AppState>,
    target_dir: String,
) -> Result<ExportReport, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| export::export_dir(&state, &target_dir)).await
}

#[tauri::command]
pub async fn data_import_md(
    state: State<'_, AppState>,
    dir: String,
) -> Result<ImportReport, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| export::import_dir(&state, &dir)).await
}

#[tauri::command]
pub async fn data_backup(
    state: State<'_, AppState>,
    target: String,
) -> Result<BackupReport, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| export::backup(&state, &target)).await
}

#[tauri::command]
pub async fn data_restore(
    state: State<'_, AppState>,
    path: String,
) -> Result<ImportReport, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| export::restore(&state, &path)).await
}