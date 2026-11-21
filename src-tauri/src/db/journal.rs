use base64::Engine;
use rusqlite::{Connection, OptionalExtension, Row, Transaction, params, params_from_iter};
use serde::Serialize;

use crate::domain::{day_key, title, utc_now_iso, words};
use crate::error::AppError;

#[derive(Debug, Serialize)]
pub struct TagDto {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct JournalSummary {
    pub id: String,
    pub title: String,
    pub preview: String,
    pub mood: Option<String>,
    pub weather: Option<String>,
    pub recorded_at: String,
    pub recorded_off: i32,
    pub day_key: String,
    pub words: i64,
    pub asset_n: i64,
    pub audio_n: i64,
    pub video_n: i64,
    pub cover_asset: Option<String>,
    pub pinned: bool,
    pub sensitive: bool,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Journal {
    pub id: String,
    pub content: String,
    pub format: String,
    pub title: String,
    pub preview: String,
    pub mood: Option<String>,
    pub weather: Option<String>,
    pub recorded_at: String,
    pub recorded_off: i32,
    pub day_key: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub pinned: bool,
    pub sensitive: bool,
    pub words: i64,
    pub asset_n: i64,
    pub audio_n: i64,
    pub video_n: i64,
    pub cover_asset: Option<String>,
    pub revision: i64,
    pub tags: Vec<TagDto>,
}

#[derive(Debug, Serialize)]
pub struct JournalPage {
    pub items: Vec<JournalSummary>,
    pub next_cursor: Option<String>,
}

pub struct SaveInput {
    pub id: Option<String>,
    pub content: String,
    pub format: Option<String>,
    pub title: Option<String>,
    pub mood: Option<String>,
    pub weather: Option<String>,
    pub tag_ids: Vec<String>,
    pub recorded_at: Option<String>,
    pub recorded_off: Option<i32>,
    pub pinned: bool,
    pub sensitive: bool,
    pub revision: Option<i64>,
}

const SUMMARY_COLS: &str = "id, content, title, mood, weather, recorded_at, recorded_off, \
                            day_key, words, asset_n, audio_n, video_n, cover_asset, pinned, \
                            sensitive, deleted_at";

const WEATHER_CODES: [&str; 17] = [
    "cloudy",
    "dust",
    "fog",
    "hail",
    "hazy",
    "hurricane",
    "lightning",
    "lightning-rainy",
    "pouring",
    "rainy",
    "sleet",
    "snow",
    "sunny",
    "sunny-off",
    "tornado",
    "ultraviolet",
    "windy",
];

fn escape_like(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

pub fn encode_cursor(pinned: bool, recorded_at: &str, id: &str) -> String {
    let flag = if pinned { 1 } else { 0 };
    base64::engine::general_purpose::STANDARD.encode(format!("{flag}|{recorded_at}|{id}"))
}

fn decode_cursor(cursor: &str) -> Option<(bool, String, String)> {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(cursor)
        .ok()?;
    let text = String::from_utf8(raw).ok()?;
    let mut parts = text.splitn(3, '|');
    let flag = parts.next()?;
    let ts = parts.next()?;
    let id = parts.next()?;
    Some((flag == "1", ts.to_string(), id.to_string()))
}

fn map_summary(row: &Row<'_>) -> rusqlite::Result<JournalSummary> {
    map_summary_with_needle(row, None)
}

fn map_summary_with_needle(
    row: &Row<'_>,
    needle: Option<&str>,
) -> rusqlite::Result<JournalSummary> {
    let content: String = row.get("content")?;
    let stored_title: Option<String> = row.get("title")?;
    let derived = title::derive_title(&content);
    let resolved_title = stored_title
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(derived);
    let preview = match needle {
        Some(text) => title::preview_around(&content, text),
        None => title::preview(&content),
    };
    Ok(JournalSummary {
        id: row.get("id")?,
        title: resolved_title,
        preview,
        mood: row.get("mood")?,
        weather: row.get("weather")?,
        recorded_at: row.get("recorded_at")?,
        recorded_off: row.get("recorded_off")?,
        day_key: row.get("day_key")?,
        words: row.get("words")?,
        asset_n: row.get("asset_n")?,
        audio_n: row.get("audio_n")?,
        video_n: row.get("video_n")?,
        cover_asset: row.get("cover_asset")?,
        pinned: row.get::<_, i64>("pinned")? != 0,
        sensitive: row.get::<_, i64>("sensitive")? != 0,
        deleted_at: row.get("deleted_at")?,
    })
}

pub struct ListParams {
    pub day_key: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub tag_id: Option<String>,
    pub mood: Option<String>,
    pub include_trash: bool,
    pub limit: i64,
    pub cursor: Option<String>,
}

pub fn list(conn: &Connection, params: ListParams) -> Result<JournalPage, AppError> {
    let mut sql = format!("SELECT {SUMMARY_COLS} FROM journal WHERE 1 = 1");
    let mut args: Vec<rusqlite::types::Value> = Vec::new();

    if params.include_trash {
        sql.push_str(" AND deleted_at IS NOT NULL");
    } else {
        sql.push_str(" AND deleted_at IS NULL");
    }
    if let Some(day) = &params.day_key {
        sql.push_str(" AND day_key = ?");
        args.push(day.clone().into());
    }
    if let Some(from) = &params.from {
        sql.push_str(" AND day_key >= ?");
        args.push(from.clone().into());
    }
    if let Some(to) = &params.to {
        sql.push_str(" AND day_key <= ?");
        args.push(to.clone().into());
    }
    if let Some(mood) = &params.mood {
        sql.push_str(" AND mood = ?");
        args.push(mood.clone().into());
    }
    if let Some(tag_id) = &params.tag_id {
        sql.push_str(
            " AND EXISTS (SELECT 1 FROM journal_tag jt \
             WHERE jt.journal_id = journal.id AND jt.tag_id = ?)",
        );
        args.push(tag_id.clone().into());
    }
    if let Some(cursor) = &params.cursor
        && let Some((pinned, ts, id)) = decode_cursor(cursor)
    {
        sql.push_str(
            " AND (pinned < ? OR (pinned = ? AND \
             (recorded_at < ? OR (recorded_at = ? AND id < ?))))",
        );
        let flag = pinned as i64;
        args.push(flag.into());
        args.push(flag.into());
        args.push(ts.clone().into());
        args.push(ts.into());
        args.push(id.into());
    }

    let limit = params.limit.clamp(1, 200);
    sql.push_str(" ORDER BY pinned DESC, recorded_at DESC, id DESC LIMIT ?");
    args.push((limit + 1).into());

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(args), map_summary)?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    let next_cursor = if items.len() as i64 > limit {
        items.truncate(limit as usize);
        items
            .last()
            .map(|item| encode_cursor(item.pinned, &item.recorded_at, &item.id))
    } else {
        None
    };
    Ok(JournalPage { items, next_cursor })
}

fn load_tags(conn: &Connection, journal_id: &str) -> rusqlite::Result<Vec<TagDto>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name FROM tag t \
         JOIN journal_tag jt ON jt.tag_id = t.id \
         WHERE jt.journal_id = ?1 ORDER BY t.name",
    )?;
    let rows = stmt.query_map([journal_id], |row| {
        Ok(TagDto {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;
    rows.collect()
}

pub fn get(conn: &Connection, id: &str) -> Result<Journal, AppError> {
    let found = conn
        .query_row(
            "SELECT id, content, format, title, mood, weather, recorded_at, recorded_off, day_key, \
             created_at, updated_at, deleted_at, pinned, sensitive, words, asset_n, audio_n, \
             video_n, cover_asset, revision FROM journal WHERE id = ?1",
            [id],
            |row| {
                let content: String = row.get("content")?;
                let stored_title: Option<String> = row.get("title")?;
                let resolved_title = stored_title
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| title::derive_title(&content));
                Ok(Journal {
                    id: row.get("id")?,
                    content: content.clone(),
                    format: row.get("format")?,
                    title: resolved_title,
                    preview: title::preview(&content),
                    mood: row.get("mood")?,
                    weather: row.get("weather")?,
                    recorded_at: row.get("recorded_at")?,
                    recorded_off: row.get("recorded_off")?,
                    day_key: row.get("day_key")?,
                    created_at: row.get("created_at")?,
                    updated_at: row.get("updated_at")?,
                    deleted_at: row.get("deleted_at")?,
                    pinned: row.get::<_, i64>("pinned")? != 0,
                    sensitive: row.get::<_, i64>("sensitive")? != 0,
                    words: row.get("words")?,
                    asset_n: row.get("asset_n")?,
                    audio_n: row.get("audio_n")?,
                    video_n: row.get("video_n")?,
                    cover_asset: row.get("cover_asset")?,
                    revision: row.get("revision")?,
                    tags: Vec::new(),
                })
            },
        )
        .optional()?;

    let mut journal = found.ok_or_else(|| AppError::new("journal_not_found", "日记不存在"))?;
    journal.tags = load_tags(conn, &journal.id)?;
    Ok(journal)
}

fn adjust_day_mark(
    conn: &Connection,
    day: &str,
    n_delta: i64,
    words_delta: i64,
) -> Result<(), AppError> {
    let changed = conn.execute(
        "UPDATE day_mark SET n = MAX(0, n + ?1), words = MAX(0, words + ?2) WHERE day_key = ?3",
        params![n_delta, words_delta, day],
    )?;
    if changed == 0 && (n_delta > 0 || words_delta > 0) {
        conn.execute(
            "INSERT INTO day_mark(day_key, n, words) VALUES(?1, ?2, ?3)",
            params![day, n_delta.max(0), words_delta.max(0)],
        )?;
    } else if changed > 0 {
        conn.execute(
            "DELETE FROM day_mark WHERE day_key = ?1 AND n <= 0 AND words <= 0",
            [day],
        )?;
    }
    Ok(())
}

fn replace_tags(
    tx: &Transaction<'_>,
    journal_id: &str,
    tag_ids: &[String],
) -> Result<(), AppError> {
    tx.execute(
        "DELETE FROM journal_tag WHERE journal_id = ?1",
        [journal_id],
    )?;
    for tag_id in tag_ids {
        tx.execute(
            "INSERT OR IGNORE INTO journal_tag(journal_id, tag_id) \
             SELECT ?1, id FROM tag WHERE id = ?2",
            params![journal_id, tag_id],
        )?;
    }
    Ok(())
}

fn scan_prefix(content: &str, prefix: &str) -> (i64, Option<String>) {
    let mut count = 0;
    let mut first: Option<String> = None;
    let mut rest = content;
    while let Some(pos) = rest.find(prefix) {
        let after = &rest[pos + prefix.len()..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'))
            .unwrap_or(after.len());
        if end > 0 {
            count += 1;
            if first.is_none() {
                first = Some(after[..end].to_string());
            }
        }
        rest = &after[end..];
    }
    (count, first)
}

pub fn scan_assets(content: &str) -> (i64, i64, i64, Option<String>) {
    let (images, cover) = scan_prefix(content, "ddasset://image/");
    let (audios, _) = scan_prefix(content, "ddasset://audio/");
    let (videos, _) = scan_prefix(content, "ddasset://video/");
    (images, audios, videos, cover)
}

pub fn save(conn: &mut Connection, input: SaveInput) -> Result<Journal, AppError> {
    let now = utc_now_iso();
    let offset = input
        .recorded_off
        .unwrap_or_else(crate::domain::local_offset_minutes);
    let recorded_at = input.recorded_at.unwrap_or_else(|| now.clone());
    let boundary = crate::db::settings::get_all(conn)?.day_boundary_minutes;
    let day = day_key::compute_day_key(&recorded_at, offset, boundary)?;
    let format = input.format.unwrap_or_else(|| "markdown".to_string());
    if format != "markdown" && format != "plain" {
        return Err(AppError::new("bad_format", "不支持的正文格式"));
    }
    if let Some(code) = input.weather.as_deref()
        && !WEATHER_CODES.contains(&code)
    {
        return Err(AppError::new("bad_weather", "不支持的天气"));
    }
    let word_count = words::count_words(&input.content);
    let (asset_n, audio_n, video_n, cover_asset) = scan_assets(&input.content);
    let explicit_title = input
        .title
        .and_then(|value| (!value.trim().is_empty()).then_some(value.trim().to_string()));

    let tx = conn.transaction()?;

    let id = if let Some(id) = input.id {
        let existing = tx
            .query_row(
                "SELECT day_key, words, deleted_at, revision FROM journal WHERE id = ?1",
                [&id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .optional()?;
        let Some((old_day, old_words, deleted_at, old_revision)) = existing else {
            return Err(AppError::new("journal_not_found", "日记不存在"));
        };
        if deleted_at.is_some() {
            return Err(AppError::new("journal_not_found", "日记已在回收站"));
        }
        if let Some(revision) = input.revision
            && revision != old_revision
        {
            return Err(AppError::new("bad_revision", "内容已在别处更新"));
        }

        tx.execute(
            "UPDATE journal SET content = ?1, format = ?2, title = ?3, mood = ?4, \
             weather = ?5, recorded_at = ?6, recorded_off = ?7, day_key = ?8, \
             updated_at = ?9, pinned = ?10, sensitive = ?11, words = ?12, asset_n = ?13, \
             audio_n = ?14, video_n = ?15, cover_asset = ?16, revision = revision + 1 \
             WHERE id = ?17",
            params![
                input.content,
                format,
                explicit_title,
                input.mood,
                input.weather,
                recorded_at,
                offset,
                day,
                now,
                input.pinned as i64,
                input.sensitive as i64,
                word_count,
                asset_n,
                audio_n,
                video_n,
                cover_asset,
                id
            ],
        )?;

        if old_day == day {
            adjust_day_mark(&tx, &day, 0, word_count - old_words)?;
        } else {
            adjust_day_mark(&tx, &old_day, -1, -old_words)?;
            adjust_day_mark(&tx, &day, 1, word_count)?;
        }
        id
    } else {
        let id = uuid::Uuid::now_v7().to_string();
        tx.execute(
            "INSERT INTO journal(id, content, format, title, mood, weather, recorded_at, \
             recorded_off, day_key, created_at, updated_at, pinned, sensitive, words, asset_n, \
             audio_n, video_n, cover_asset, revision) \
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10, ?11, ?12, ?13, ?14, ?15, \
             ?16, ?17, 1)",
            params![
                id,
                input.content,
                format,
                explicit_title,
                input.mood,
                input.weather,
                recorded_at,
                offset,
                day,
                now,
                input.pinned as i64,
                input.sensitive as i64,
                word_count,
                asset_n,
                audio_n,
                video_n,
                cover_asset
            ],
        )?;
        adjust_day_mark(&tx, &day, 1, word_count)?;
        id
    };

    replace_tags(&tx, &id, &input.tag_ids)?;
    let journal = get(&tx, &id)?;
    tx.commit()?;
    Ok(journal)
}

pub fn soft_delete(conn: &mut Connection, id: &str) -> Result<String, AppError> {
    let tx = conn.transaction()?;
    let existing = tx
        .query_row(
            "SELECT day_key, words, deleted_at FROM journal WHERE id = ?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((day, word_count, deleted_at)) = existing else {
        return Err(AppError::new("journal_not_found", "日记不存在"));
    };
    if deleted_at.is_none() {
        tx.execute(
            "UPDATE journal SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![utc_now_iso(), id],
        )?;
        adjust_day_mark(&tx, &day, -1, -word_count)?;
    }
    tx.commit()?;
    Ok(day)
}

pub fn restore(conn: &mut Connection, id: &str) -> Result<String, AppError> {
    let tx = conn.transaction()?;
    let existing = tx
        .query_row(
            "SELECT day_key, words, deleted_at FROM journal WHERE id = ?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((day, word_count, deleted_at)) = existing else {
        return Err(AppError::new("journal_not_found", "日记不存在"));
    };
    if deleted_at.is_some() {
        tx.execute(
            "UPDATE journal SET deleted_at = NULL, updated_at = ?1 WHERE id = ?2",
            params![utc_now_iso(), id],
        )?;
        adjust_day_mark(&tx, &day, 1, word_count)?;
    }
    tx.commit()?;
    Ok(day)
}

pub fn purge(conn: &mut Connection, id: &str) -> Result<(), AppError> {
    let changed = conn.execute(
        "DELETE FROM journal WHERE id = ?1 AND deleted_at IS NOT NULL",
        [id],
    )?;
    if changed == 0 {
        return Err(AppError::new("journal_not_found", "回收站中没有这条日记"));
    }
    Ok(())
}

pub fn empty_trash(conn: &mut Connection) -> Result<i64, AppError> {
    let count = conn.execute("DELETE FROM journal WHERE deleted_at IS NOT NULL", [])?;
    Ok(count as i64)
}

pub fn set_pinned(conn: &Connection, id: &str, value: bool) -> Result<(), AppError> {
    let changed = conn.execute(
        "UPDATE journal SET pinned = ?1, updated_at = ?2 WHERE id = ?3",
        params![value as i64, utc_now_iso(), id],
    )?;
    if changed == 0 {
        return Err(AppError::new("journal_not_found", "日记不存在"));
    }
    Ok(())
}

pub fn set_sensitive(conn: &Connection, id: &str, value: bool) -> Result<(), AppError> {
    let changed = conn.execute(
        "UPDATE journal SET sensitive = ?1, updated_at = ?2 WHERE id = ?3",
        params![value as i64, utc_now_iso(), id],
    )?;
    if changed == 0 {
        return Err(AppError::new("journal_not_found", "日记不存在"));
    }
    Ok(())
}

pub fn day_marks(conn: &Connection, from: &str, to: &str) -> Result<Vec<DayMark>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT day_key, n, words FROM day_mark WHERE day_key BETWEEN ?1 AND ?2 ORDER BY day_key",
    )?;
    let rows = stmt.query_map([from, to], |row| {
        Ok(DayMark {
            day_key: row.get(0)?,
            n: row.get(1)?,
            words: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

#[derive(Debug, Serialize)]
pub struct DayMark {
    pub day_key: String,
    pub n: i64,
    pub words: i64,
}

pub fn recompute_day_keys(conn: &Connection, boundary_minutes: u32) -> Result<(), AppError> {
    let rows: Vec<(String, String, i32)> = {
        let mut stmt = conn.prepare("SELECT id, recorded_at, recorded_off FROM journal")?;
        let mapped =
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        mapped.collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (id, recorded_at, offset) in rows {
        let day = day_key::compute_day_key(&recorded_at, offset, boundary_minutes)?;
        conn.execute(
            "UPDATE journal SET day_key = ?1 WHERE id = ?2",
            params![day, id],
        )?;
    }
    conn.execute("DELETE FROM day_mark", [])?;
    conn.execute(
        "INSERT INTO day_mark(day_key, n, words) \
         SELECT day_key, COUNT(*), COALESCE(SUM(words), 0) FROM journal \
         WHERE deleted_at IS NULL GROUP BY day_key",
        [],
    )?;
    Ok(())
}

pub fn search(
    conn: &Connection,
    text: &str,
    from: Option<&str>,
    to: Option<&str>,
    tag_id: Option<&str>,
    mood: Option<&str>,
    limit: i64,
) -> Result<(Vec<JournalSummary>, bool), AppError> {
    let limit = limit.clamp(1, 200);
    let pattern = format!("%{}%", escape_like(text));
    let mut sql = format!(
        "SELECT {SUMMARY_COLS} FROM journal WHERE deleted_at IS NULL \
         AND (lower(content) LIKE lower(?1) ESCAPE '\\' \
              OR lower(coalesce(title, '')) LIKE lower(?1) ESCAPE '\\')"
    );
    let mut args: Vec<rusqlite::types::Value> = vec![pattern.into()];
    if let Some(from) = from {
        sql.push_str(" AND day_key >= ?");
        args.push(from.to_string().into());
    }
    if let Some(to) = to {
        sql.push_str(" AND day_key <= ?");
        args.push(to.to_string().into());
    }
    if let Some(tag_id) = tag_id {
        sql.push_str(
            " AND EXISTS (SELECT 1 FROM journal_tag jt \
             WHERE jt.journal_id = journal.id AND jt.tag_id = ?)",
        );
        args.push(tag_id.to_string().into());
    }
    if let Some(mood) = mood {
        sql.push_str(" AND mood = ?");
        args.push(mood.to_string().into());
    }
    sql.push_str(" ORDER BY recorded_at DESC, id DESC LIMIT ?");
    args.push((limit + 1).into());

    let mut stmt = conn.prepare(&sql)?;
    let mut items = Vec::new();
    {
        let rows = stmt.query_map(params_from_iter(args), |row| {
            map_summary_with_needle(row, Some(text))
        })?;
        for row in rows {
            items.push(row?);
        }
    }
    let partial = items.len() as i64 > limit;
    if partial {
        items.truncate(limit as usize);
    }
    Ok((items, partial))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{migrate, settings, tag};

    fn test_conn() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        migrate::run(&mut conn).expect("migrations run");
        conn
    }

    fn input(content: &str) -> SaveInput {
        SaveInput {
            id: None,
            content: content.to_string(),
            format: None,
            title: None,
            mood: None,
            weather: None,
            tag_ids: Vec::new(),
            recorded_at: Some("2026-11-21T10:00:00Z".to_string()),
            recorded_off: Some(480),
            pinned: false,
            sensitive: false,
            revision: None,
        }
    }

    fn list_day(conn: &Connection, day: &str) -> JournalPage {
        list(
            conn,
            ListParams {
                day_key: Some(day.to_string()),
                from: None,
                to: None,
                tag_id: None,
                mood: None,
                include_trash: false,
                limit: 20,
                cursor: None,
            },
        )
        .expect("list day")
    }

    #[test]
    fn save_derives_title_and_day_key() {
        let mut conn = test_conn();
        let saved = save(&mut conn, input("# 第一行标题\n\n正文内容")).expect("save");
        assert_eq!(saved.day_key, "2026-11-21");
        assert_eq!(saved.title, "第一行标题");

        let page = list_day(&conn, "2026-11-21");
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].id, saved.id);

        let marks = day_marks(&conn, "2026-11-01", "2026-11-30").expect("marks");
        assert_eq!(marks.len(), 1);
        assert_eq!(marks[0].n, 1);
    }

    #[test]
    fn save_derives_media_counts_by_kind() {
        let mut conn = test_conn();
        let saved = save(
            &mut conn,
            input(
                "![a](ddasset://image/a.png)\n\n![b](ddasset://audio/b.mp3)\n\n\
                 ![c](ddasset://video/c.mp4)\n\n![d](ddasset://image/d.png)",
            ),
        )
        .expect("save");
        assert_eq!(saved.asset_n, 2);
        assert_eq!(saved.audio_n, 1);
        assert_eq!(saved.video_n, 1);
        assert_eq!(saved.cover_asset.as_deref(), Some("a.png"));

        let page = list_day(&conn, "2026-11-21");
        assert_eq!(page.items[0].audio_n, 1);
        assert_eq!(page.items[0].video_n, 1);
    }

    #[test]
    fn list_puts_pinned_first_and_paginates_across_pin_groups() {
        let mut conn = test_conn();
        let mut old = input("oldest");
        old.recorded_at = Some("2026-11-21T02:00:00Z".to_string());
        let old = save(&mut conn, old).expect("save old");
        let mut middle = input("middle");
        middle.recorded_at = Some("2026-11-21T03:00:00Z".to_string());
        let middle = save(&mut conn, middle).expect("save middle");
        let mut newest = input("newest");
        newest.recorded_at = Some("2026-11-21T04:00:00Z".to_string());
        let newest = save(&mut conn, newest).expect("save newest");
        set_pinned(&conn, &old.id, true).expect("pin");

        let page = list_day(&conn, "2026-11-21");
        let ids: Vec<&str> = page.items.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![old.id.as_str(), newest.id.as_str(), middle.id.as_str()]
        );

        let first = list(
            &conn,
            ListParams {
                day_key: Some("2026-11-21".to_string()),
                from: None,
                to: None,
                tag_id: None,
                mood: None,
                include_trash: false,
                limit: 2,
                cursor: None,
            },
        )
        .expect("first page");
        assert_eq!(first.items.len(), 2);

        let second = list(
            &conn,
            ListParams {
                day_key: Some("2026-11-21".to_string()),
                from: None,
                to: None,
                tag_id: None,
                mood: None,
                include_trash: false,
                limit: 2,
                cursor: first.next_cursor,
            },
        )
        .expect("second page");
        assert_eq!(second.items.len(), 1);
        assert_eq!(second.items[0].id, middle.id);
    }

    #[test]
    fn boundary_rebuckets_late_night_entries_and_marks() {
        let mut conn = test_conn();
        let mut late = input("late night");
        late.recorded_at = Some("2026-11-21T17:30:00Z".to_string());
        late.recorded_off = Some(480);
        let late = save(&mut conn, late).expect("save late");
        assert_eq!(late.day_key, "2026-11-22");

        settings::patch(&mut conn, serde_json::json!({ "dayBoundaryMinutes": 240 }))
            .expect("patch boundary");

        let moved = get(&conn, &late.id).expect("get moved");
        assert_eq!(moved.day_key, "2026-11-21");
        assert_eq!(list_day(&conn, "2026-11-21").items.len(), 1);
        assert!(list_day(&conn, "2026-11-22").items.is_empty());
        let marks = day_marks(&conn, "2026-11-01", "2026-11-30").expect("marks");
        assert_eq!(marks.len(), 1);
        assert_eq!(marks[0].day_key, "2026-11-21");

        let mut after = input("after boundary");
        after.recorded_at = Some("2026-11-21T17:45:00Z".to_string());
        after.recorded_off = Some(480);
        let after = save(&mut conn, after).expect("save after");
        assert_eq!(after.day_key, "2026-11-21");
    }

    #[test]
    fn update_checks_revision_and_recomputes_words() {
        let mut conn = test_conn();
        let saved = save(&mut conn, input("hello world")).expect("save");
        let revision = saved.revision;

        let mut stale = input("changed once");
        stale.id = Some(saved.id.clone());
        stale.revision = Some(revision + 1);
        let err = save(&mut conn, stale).expect_err("stale revision rejected");
        assert_eq!(err.code, "bad_revision");

        let mut ok = input("hello world again");
        ok.id = Some(saved.id.clone());
        ok.revision = Some(revision);
        let updated = save(&mut conn, ok).expect("update");
        assert_eq!(updated.revision, revision + 1);
        assert!(updated.content.contains("again"));
    }

    #[test]
    fn soft_delete_restore_and_purge_keep_day_marks_in_sync() {
        let mut conn = test_conn();
        let saved = save(&mut conn, input("今天写了一点东西")).expect("save");

        let day = soft_delete(&mut conn, &saved.id).expect("soft delete");
        assert_eq!(day, "2026-11-21");
        assert_eq!(list_day(&conn, &day).items.len(), 0);
        assert!(
            day_marks(&conn, "2026-11-01", "2026-11-30")
                .expect("marks")
                .is_empty()
        );

        let trash = list(
            &conn,
            ListParams {
                day_key: None,
                from: None,
                to: None,
                tag_id: None,
                mood: None,
                include_trash: true,
                limit: 20,
                cursor: None,
            },
        )
        .expect("trash list");
        assert_eq!(trash.items.len(), 1);

        restore(&mut conn, &saved.id).expect("restore");
        assert_eq!(list_day(&conn, &day).items.len(), 1);

        soft_delete(&mut conn, &saved.id).expect("soft delete again");
        purge(&mut conn, &saved.id).expect("purge");
        assert!(get(&conn, &saved.id).is_err());
    }

    #[test]
    fn search_matches_single_cjk_character_and_escapes_like() {
        let mut conn = test_conn();
        save(&mut conn, input("昨晚梦见一只猫")).expect("save");
        save(&mut conn, input("100% 完成")).expect("save");

        let (items, partial) = search(&conn, "梦", None, None, None, None, 20).expect("search");
        assert!(!partial);
        assert_eq!(items.len(), 1);

        let (none, _) =
            search(&conn, "%", None, None, None, None, 20).expect("search escaped");
        assert_eq!(none.len(), 1);
        assert!(none[0].preview.contains("100%"));
    }

    #[test]
    fn tag_filter_and_usage_count_follow_live_entries() {
        let mut conn = test_conn();
        let tag = tag::create(&conn, "旅行").expect("create tag");
        let mut with_tag = input("去山上看云");
        with_tag.tag_ids = vec![tag.id.clone()];
        let saved = save(&mut conn, with_tag).expect("save with tag");

        let page = list(
            &conn,
            ListParams {
                day_key: None,
                from: None,
                to: None,
                tag_id: Some(tag.id.clone()),
                mood: None,
                include_trash: false,
                limit: 20,
                cursor: None,
            },
        )
        .expect("tag list");
        assert_eq!(page.items.len(), 1);
        assert_eq!(tag::list(&conn).expect("tags")[0].usage, 1);

        soft_delete(&mut conn, &saved.id).expect("soft delete");
        assert_eq!(tag::list(&conn).expect("tags")[0].usage, 0);
    }
}
