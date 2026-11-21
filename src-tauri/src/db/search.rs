use rusqlite::Connection;
use serde::Serialize;

use crate::db::journal::{self, JournalSummary, ListParams};
use crate::error::AppError;

#[derive(Debug, Serialize)]
pub struct SearchPage {
    pub items: Vec<JournalSummary>,
    pub partial: bool,
}

pub struct SearchParams {
    pub text: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub tag_id: Option<String>,
    pub mood: Option<String>,
    pub limit: i64,
}

pub fn query(conn: &Connection, params: SearchParams) -> Result<SearchPage, AppError> {
    let text = params.text.trim().to_string();
    if text.is_empty() {
        let page = journal::list(
            conn,
            ListParams {
                day_key: None,
                from: params.from,
                to: params.to,
                tag_id: params.tag_id,
                mood: params.mood,
                include_trash: false,
                limit: params.limit,
                cursor: None,
            },
        )?;
        return Ok(SearchPage {
            items: page.items,
            partial: false,
        });
    }

    let (items, partial) = journal::search(
        conn,
        &text,
        params.from.as_deref(),
        params.to.as_deref(),
        params.tag_id.as_deref(),
        params.mood.as_deref(),
        params.limit,
    )?;
    Ok(SearchPage { items, partial })
}
