use tauri::State;

use crate::{
    commands::run_blocking,
    error::AppError,
    fs::assets::{self, AssetDto, ImportFile},
    state::AppState,
};

#[tauri::command]
pub async fn media_import(
    state: State<'_, AppState>,
    files: Vec<ImportFile>,
) -> Result<Vec<AssetDto>, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| assets::import_files(&state, files)).await
}

#[tauri::command]
pub async fn asset_gc(state: State<'_, AppState>, names: Vec<String>) -> Result<usize, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| assets::gc_unused(&state, names)).await
}