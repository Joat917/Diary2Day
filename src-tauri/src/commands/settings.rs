use serde_json::Value;
use tauri::{Emitter, State};

use crate::{
    commands::run_blocking_public,
    db::settings::{self, Settings},
    error::AppError,
    events::SETTINGS_CHANGED,
    state::AppState,
};

#[tauri::command]
pub async fn settings_get_all(state: State<'_, AppState>) -> Result<Settings, AppError> {
    let st = state.inner().clone();
    run_blocking_public(st, move |state| {
        state.with_db(|conn| settings::get_all(conn))
    })
    .await
}

#[tauri::command]
pub async fn settings_patch(
    state: State<'_, AppState>,
    patch: Value,
) -> Result<Settings, AppError> {
    let st = state.inner().clone();
    let result = run_blocking_public(st.clone(), move |state| {
        state.with_db(|conn| settings::patch(conn, patch))
    })
    .await?;
    let _ = st.app.emit(SETTINGS_CHANGED, &result);
    Ok(result)
}
