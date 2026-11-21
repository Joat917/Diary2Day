use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::AppError;

#[derive(Clone, Debug)]
pub struct LockRow {
    pub salt: String,
    pub hash: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LockState {
    pub enabled: bool,
    pub locked: bool,
}

pub fn load(conn: &Connection) -> Result<Option<LockRow>, AppError> {
    Ok(conn
        .query_row(
            "SELECT salt, hash FROM app_lock WHERE id = 1 AND enabled = 1",
            [],
            |row| {
                Ok(LockRow {
                    salt: row.get(0)?,
                    hash: row.get(1)?,
                })
            },
        )
        .optional()?)
}

pub fn derive(salt: &str, code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(code.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn save(conn: &Connection, salt: &str, hash: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO app_lock(id, salt, hash, enabled) VALUES(1, ?1, ?2, 1) \
         ON CONFLICT(id) DO UPDATE SET salt = excluded.salt, hash = excluded.hash, enabled = 1",
        params![salt, hash],
    )?;
    Ok(())
}

pub fn clear(conn: &Connection) -> Result<(), AppError> {
    conn.execute("DELETE FROM app_lock WHERE id = 1", [])?;
    Ok(())
}

pub fn verify(row: &LockRow, code: &str) -> bool {
    derive(&row.salt, code) == row.hash
}

pub fn validate_code(code: &str) -> Result<(), AppError> {
    let trimmed = code.trim();
    if trimmed.chars().count() < 4 {
        return Err(AppError::new("bad_lock_code", "口令至少需要 4 个字符"));
    }
    if trimmed.chars().count() > 64 || code != trimmed {
        return Err(AppError::new("bad_lock_code", "口令格式不合法"));
    }
    Ok(())
}