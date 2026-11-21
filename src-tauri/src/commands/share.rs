use tauri::State;

use crate::{
    commands::run_blocking,
    error::AppError,
    share::{self, ShareReport},
    state::AppState,
};

#[tauri::command]
pub async fn share_drain(state: State<'_, AppState>) -> Result<Option<ShareReport>, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| share::drain(&state)).await
}