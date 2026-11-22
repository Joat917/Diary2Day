use tauri::State;

use crate::{commands::run_blocking, error::AppError, export, state::AppState};

#[tauri::command]
pub async fn image_export(
    state: State<'_, AppState>,
    target: String,
    data: String,
) -> Result<u64, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |_| export::write_image(&target, &data)).await
}