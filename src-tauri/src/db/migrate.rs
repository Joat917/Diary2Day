use rusqlite::Connection;

use crate::error::AppError;

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("schema/0001_init.sql")),
    (2, include_str!("schema/0002_m2.sql")),
    (3, include_str!("schema/0003_media_counts.sql")),
];

pub fn run(conn: &mut Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
           version    INTEGER PRIMARY KEY,
           name       TEXT NOT NULL,
           applied_at TEXT NOT NULL
         );",
    )?;

    let current: i64 = conn.query_row(
        "SELECT coalesce(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )?;

    for (version, sql) in MIGRATIONS {
        if *version <= current {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(sql)
            .map_err(|err| AppError::sqlite(err).with_detail(format!("migration {version}")))?;
        tx.execute(
            "INSERT INTO schema_version(version, name, applied_at) VALUES(?1, ?2, ?3)",
            rusqlite::params![
                version,
                format!("{version:04}_init"),
                crate::domain::utc_now_iso()
            ],
        )?;
        tx.commit()?;
    }
    Ok(())
}
