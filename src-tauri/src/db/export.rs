use rusqlite::{Connection, params};
use serde::Serialize;

use crate::error::AppError;

#[derive(Debug, Serialize)]
pub struct ExportEntry {
    pub id: String,
    pub content: String,
    pub format: String,
    pub mood: Option<String>,
    pub weather: Option<String>,
    pub recorded_at: String,
    pub recorded_off: i32,
    pub day_key: String,
    pub pinned: bool,
    pub sensitive: bool,
    pub words: i64,
    pub tags: Vec<String>,
}

pub fn live_entries(conn: &Connection) -> Result<Vec<ExportEntry>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, content, format, mood, weather, recorded_at, recorded_off, day_key, pinned, \
         sensitive, words FROM journal WHERE deleted_at IS NULL \
         ORDER BY day_key, recorded_at, id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ExportEntry {
            id: row.get(0)?,
            content: row.get(1)?,
            format: row.get(2)?,
            mood: row.get(3)?,
            weather: row.get(4)?,
            recorded_at: row.get(5)?,
            recorded_off: row.get(6)?,
            day_key: row.get(7)?,
            pinned: row.get::<_, i64>(8)? != 0,
            sensitive: row.get::<_, i64>(9)? != 0,
            words: row.get(10)?,
            tags: Vec::new(),
        })
    })?;
    let mut items = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    let mut tag_stmt = conn.prepare(
        "SELECT t.name FROM tag t JOIN journal_tag jt ON jt.tag_id = t.id \
         WHERE jt.journal_id = ?1 ORDER BY t.name",
    )?;
    for item in &mut items {
        let tags = tag_stmt.query_map([&item.id], |row| row.get::<_, String>(0))?;
        item.tags = tags.collect::<rusqlite::Result<Vec<_>>>()?;
    }
    Ok(items)
}

pub fn schema_version(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT coalesce(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

pub fn insert_asset_if_missing(
    conn: &Connection,
    name: &str,
    kind: &str,
    ext: &str,
    size: i64,
    orig_name: &str,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT OR IGNORE INTO asset(name, kind, ext, size, width, height, duration_ms, hash, \
         orig_name, created_at) VALUES(?1, ?2, ?3, ?4, NULL, NULL, NULL, NULL, ?5, ?6)",
        params![name, kind, ext, size, orig_name, crate::domain::utc_now_iso()],
    )?;
    Ok(())
}