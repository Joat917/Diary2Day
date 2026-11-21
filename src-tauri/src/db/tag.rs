use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::domain::name_key;
use crate::error::AppError;

#[derive(Debug, Serialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub usage: i64,
}

pub fn list(conn: &Connection) -> Result<Vec<Tag>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, \
         (SELECT COUNT(*) FROM journal_tag jt JOIN journal j ON j.id = jt.journal_id \
          WHERE jt.tag_id = t.id AND j.deleted_at IS NULL) \
         FROM tag t ORDER BY t.name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            usage: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn create(conn: &Connection, name: &str) -> Result<Tag, AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::new("bad_tag", "标签名不能为空"));
    }
    let key = name_key::fold(trimmed);
    let duplicate: Option<String> = conn
        .query_row("SELECT id FROM tag WHERE name_key = ?1", [&key], |row| {
            row.get(0)
        })
        .optional()?;
    if duplicate.is_some() {
        return Err(AppError::new("tag_exists", "同名标签已存在"));
    }
    let id = uuid::Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO tag(id, name, name_key, created_at) VALUES(?1, ?2, ?3, ?4)",
        params![id, trimmed, key, crate::domain::utc_now_iso()],
    )?;
    Ok(Tag {
        id,
        name: trimmed.to_string(),
        usage: 0,
    })
}

pub fn rename(conn: &Connection, id: &str, name: &str) -> Result<(), AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::new("bad_tag", "标签名不能为空"));
    }
    let key = name_key::fold(trimmed);
    let duplicate: Option<String> = conn
        .query_row(
            "SELECT id FROM tag WHERE name_key = ?1 AND id <> ?2",
            params![key, id],
            |row| row.get(0),
        )
        .optional()?;
    if duplicate.is_some() {
        return Err(AppError::new("tag_exists", "同名标签已存在"));
    }
    let changed = conn.execute(
        "UPDATE tag SET name = ?1, name_key = ?2 WHERE id = ?3",
        params![trimmed, key, id],
    )?;
    if changed == 0 {
        return Err(AppError::new("tag_not_found", "标签不存在"));
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM tag WHERE id = ?1", [id])?;
    Ok(())
}

pub fn merge(conn: &mut Connection, source: &str, target: &str) -> Result<(), AppError> {
    if source == target {
        return Ok(());
    }
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT OR IGNORE INTO journal_tag(journal_id, tag_id) \
         SELECT journal_id, ?1 FROM journal_tag WHERE tag_id = ?2",
        params![target, source],
    )?;
    tx.execute("DELETE FROM journal_tag WHERE tag_id = ?1", [source])?;
    tx.execute("DELETE FROM tag WHERE id = ?1", [source])?;
    tx.commit()?;
    Ok(())
}
