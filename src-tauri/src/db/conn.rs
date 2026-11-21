use std::path::Path;

use rusqlite::Connection;

use crate::error::AppError;

pub fn open(path: &Path) -> Result<Connection, AppError> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=4000;
         PRAGMA synchronous=NORMAL;",
    )?;
    Ok(conn)
}
