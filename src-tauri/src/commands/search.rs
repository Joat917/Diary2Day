use tauri::State;

use crate::{
    commands::run_blocking,
    db::search::{self, SearchPage, SearchParams},
    error::AppError,
    state::AppState,
};

#[tauri::command]
pub async fn search_query(
    state: State<'_, AppState>,
    text: String,
    from: Option<String>,
    to: Option<String>,
    tag_id: Option<String>,
    mood: Option<String>,
    limit: Option<i64>,
) -> Result<SearchPage, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| {
        state.with_db(|conn| {
            search::query(
                conn,
                SearchParams {
                    text,
                    from,
                    to,
                    tag_id,
                    mood,
                    limit: limit.unwrap_or(50),
                },
            )
        })
    })
    .await
}
