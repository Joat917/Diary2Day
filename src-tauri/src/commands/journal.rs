use tauri::{Emitter, State};

use crate::{
    commands::run_blocking,
    db::journal::{self, DayMark, Journal, JournalPage, ListParams, SaveInput},
    error::AppError,
    events::{self, DataChanged},
    state::AppState,
};

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn journal_list(
    state: State<'_, AppState>,
    day_key: Option<String>,
    from: Option<String>,
    to: Option<String>,
    tag_id: Option<String>,
    mood: Option<String>,
    include_trash: Option<bool>,
    limit: Option<i64>,
    cursor: Option<String>,
) -> Result<JournalPage, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| {
        state.with_db(|conn| {
            journal::list(
                conn,
                ListParams {
                    day_key,
                    from,
                    to,
                    tag_id,
                    mood,
                    include_trash: include_trash.unwrap_or(false),
                    limit: limit.unwrap_or(50),
                    cursor,
                },
            )
        })
    })
    .await
}

#[tauri::command]
pub async fn journal_get(state: State<'_, AppState>, id: String) -> Result<Journal, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| {
        state.with_db(|conn| journal::get(conn, &id))
    })
    .await
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn journal_save(
    state: State<'_, AppState>,
    id: Option<String>,
    content: String,
    format: Option<String>,
    title: Option<String>,
    mood: Option<String>,
    weather: Option<String>,
    tag_ids: Option<Vec<String>>,
    recorded_at: Option<String>,
    recorded_off: Option<i32>,
    pinned: Option<bool>,
    sensitive: Option<bool>,
    revision: Option<i64>,
) -> Result<Journal, AppError> {
    let st = state.inner().clone();
    let result = run_blocking(st.clone(), move |state| {
        let input = SaveInput {
            id,
            content,
            format,
            title,
            mood,
            weather,
            tag_ids: tag_ids.unwrap_or_default(),
            recorded_at,
            recorded_off,
            pinned: pinned.unwrap_or(false),
            sensitive: sensitive.unwrap_or(false),
            revision,
        };
        state.with_db(|conn| journal::save(conn, input))
    })
    .await?;

    let _ = st.app.emit(
        events::DATA_CHANGED,
        DataChanged {
            kind: "journal".into(),
            id: Some(result.id.clone()),
            day_key: Some(result.day_key.clone()),
        },
    );
    Ok(result)
}

#[tauri::command]
pub async fn journal_delete(state: State<'_, AppState>, id: String) -> Result<String, AppError> {
    let st = state.inner().clone();
    let id_for_delete = id.clone();
    let day_key = run_blocking(st.clone(), move |state| {
        state.with_db(|conn| journal::soft_delete(conn, &id_for_delete))
    })
    .await?;
    let _ = st.app.emit(
        events::DATA_CHANGED,
        DataChanged {
            kind: "journal".into(),
            id: Some(id),
            day_key: Some(day_key.clone()),
        },
    );
    Ok(day_key)
}

#[tauri::command]
pub async fn journal_restore(state: State<'_, AppState>, id: String) -> Result<String, AppError> {
    let st = state.inner().clone();
    let id_for_restore = id.clone();
    let day_key = run_blocking(st.clone(), move |state| {
        state.with_db(|conn| journal::restore(conn, &id_for_restore))
    })
    .await?;
    let _ = st.app.emit(
        events::DATA_CHANGED,
        DataChanged {
            kind: "journal".into(),
            id: Some(id),
            day_key: Some(day_key.clone()),
        },
    );
    Ok(day_key)
}

#[tauri::command]
pub async fn journal_purge(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| {
        state.with_db(|conn| journal::purge(conn, &id))
    })
    .await
}

#[tauri::command]
pub async fn journal_empty_trash(state: State<'_, AppState>) -> Result<i64, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| state.with_db(journal::empty_trash)).await
}

#[tauri::command]
pub async fn journal_trash_list(
    state: State<'_, AppState>,
    limit: Option<i64>,
    cursor: Option<String>,
) -> Result<JournalPage, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| {
        state.with_db(|conn| {
            journal::list(
                conn,
                ListParams {
                    day_key: None,
                    from: None,
                    to: None,
                    tag_id: None,
                    mood: None,
                    include_trash: true,
                    limit: limit.unwrap_or(50),
                    cursor,
                },
            )
        })
    })
    .await
}

#[tauri::command]
pub async fn journal_set_pinned(
    state: State<'_, AppState>,
    id: String,
    value: bool,
) -> Result<(), AppError> {
    let st = state.inner().clone();
    let id_for_update = id.clone();
    run_blocking(st.clone(), move |state| {
        state.with_db(|conn| journal::set_pinned(conn, &id_for_update, value))
    })
    .await?;
    let _ = st.app.emit(
        events::DATA_CHANGED,
        DataChanged {
            kind: "journal".into(),
            id: Some(id),
            day_key: None,
        },
    );
    Ok(())
}

#[tauri::command]
pub async fn journal_set_sensitive(
    state: State<'_, AppState>,
    id: String,
    value: bool,
) -> Result<(), AppError> {
    let st = state.inner().clone();
    let id_for_update = id.clone();
    run_blocking(st.clone(), move |state| {
        state.with_db(|conn| journal::set_sensitive(conn, &id_for_update, value))
    })
    .await?;
    let _ = st.app.emit(
        events::DATA_CHANGED,
        DataChanged {
            kind: "journal".into(),
            id: Some(id),
            day_key: None,
        },
    );
    Ok(())
}

#[tauri::command]
pub async fn day_marks(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> Result<Vec<DayMark>, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| {
        state.with_db(|conn| journal::day_marks(conn, &from, &to))
    })
    .await
}
