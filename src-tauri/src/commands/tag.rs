use tauri::{Emitter, State};

use crate::{
    commands::run_blocking,
    db::tag::{self, Tag},
    error::AppError,
    events::{self, DataChanged},
    state::AppState,
};

fn emit_tags_changed(state: &AppState) {
    let _ = state.app.emit(
        events::DATA_CHANGED,
        DataChanged {
            kind: "tag".into(),
            id: None,
            day_key: None,
        },
    );
}

#[tauri::command]
pub async fn tag_list(state: State<'_, AppState>) -> Result<Vec<Tag>, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| state.with_db(|conn| tag::list(conn))).await
}

#[tauri::command]
pub async fn tag_create(state: State<'_, AppState>, name: String) -> Result<Tag, AppError> {
    let st = state.inner().clone();
    let result = run_blocking(st.clone(), move |state| {
        state.with_db(|conn| tag::create(conn, &name))
    })
    .await?;
    emit_tags_changed(&st);
    Ok(result)
}

#[tauri::command]
pub async fn tag_rename(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> Result<(), AppError> {
    let st = state.inner().clone();
    run_blocking(st.clone(), move |state| {
        state.with_db(|conn| tag::rename(conn, &id, &name))
    })
    .await?;
    emit_tags_changed(&st);
    Ok(())
}

#[tauri::command]
pub async fn tag_delete(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let st = state.inner().clone();
    run_blocking(st.clone(), move |state| {
        state.with_db(|conn| tag::delete(conn, &id))
    })
    .await?;
    emit_tags_changed(&st);
    Ok(())
}

#[tauri::command]
pub async fn tag_merge(
    state: State<'_, AppState>,
    source: String,
    target: String,
) -> Result<(), AppError> {
    let st = state.inner().clone();
    run_blocking(st.clone(), move |state| {
        state.with_db(|conn| tag::merge(conn, &source, &target))
    })
    .await?;
    emit_tags_changed(&st);
    Ok(())
}
