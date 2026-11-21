use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub theme: String,
    pub language: String,
    pub date_format: String,
    pub week_start: u8,
    pub indent_first_line: bool,
    pub image_lazy: bool,
    pub show_stats_card: bool,
    pub quick_record: bool,
    pub lock_leave_delay_s: u32,
    pub reduce_motion: bool,
    pub day_boundary_minutes: u32,
    pub card_time: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            language: "zh-CN".into(),
            date_format: "slash".into(),
            week_start: 1,
            indent_first_line: false,
            image_lazy: true,
            show_stats_card: true,
            quick_record: true,
            lock_leave_delay_s: 60,
            reduce_motion: false,
            day_boundary_minutes: 0,
            card_time: "recorded".into(),
        }
    }
}

const KEYS: &[&str] = &[
    "theme",
    "language",
    "dateFormat",
    "weekStart",
    "indentFirstLine",
    "imageLazy",
    "showStatsCard",
    "quickRecord",
    "lockLeaveDelayS",
    "reduceMotion",
    "dayBoundaryMinutes",
    "cardTime",
];

fn read_value(conn: &Connection, key: &str) -> Result<Option<Value>, AppError> {
    let raw: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .optional()?;
    Ok(raw.and_then(|text| serde_json::from_str(&text).ok()))
}

fn read_field<T: DeserializeOwned>(conn: &Connection, key: &str, fallback: T) -> T {
    read_value(conn, key)
        .ok()
        .flatten()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or(fallback)
}

pub fn get_all(conn: &Connection) -> Result<Settings, AppError> {
    let default = Settings::default();
    Ok(Settings {
        theme: read_field(conn, "theme", default.theme),
        language: read_field(conn, "language", default.language),
        date_format: read_field(conn, "dateFormat", default.date_format),
        week_start: read_field(conn, "weekStart", default.week_start),
        indent_first_line: read_field(conn, "indentFirstLine", default.indent_first_line),
        image_lazy: read_field(conn, "imageLazy", default.image_lazy),
        show_stats_card: read_field(conn, "showStatsCard", default.show_stats_card),
        quick_record: read_field(conn, "quickRecord", default.quick_record),
        lock_leave_delay_s: read_field(conn, "lockLeaveDelayS", default.lock_leave_delay_s),
        reduce_motion: read_field(conn, "reduceMotion", default.reduce_motion),
        day_boundary_minutes: read_field(
            conn,
            "dayBoundaryMinutes",
            default.day_boundary_minutes,
        ),
        card_time: read_field(conn, "cardTime", default.card_time),
    })
}

fn validate(settings: &Settings) -> Result<(), AppError> {
    if !matches!(settings.theme.as_str(), "system" | "light" | "dark") {
        return Err(AppError::new("bad_setting", "未知主题"));
    }
    if !matches!(settings.language.as_str(), "zh-CN" | "en") {
        return Err(AppError::new("bad_setting", "未知语言"));
    }
    if !matches!(settings.date_format.as_str(), "slash" | "short" | "zh") {
        return Err(AppError::new("bad_setting", "未知日期格式"));
    }
    if settings.week_start > 6 {
        return Err(AppError::new("bad_setting", "周起始日超出范围"));
    }
    if settings.day_boundary_minutes >= 480 {
        return Err(AppError::new("bad_setting", "每日分界点需在八点之前"));
    }
    if !matches!(settings.card_time.as_str(), "recorded" | "updated") {
        return Err(AppError::new("bad_setting", "未知卡片时间来源"));
    }
    Ok(())
}

fn write_value(conn: &Connection, key: &str, value: &Value) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO settings(key, value, updated_at) VALUES(?1, ?2, ?3) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, value.to_string(), crate::domain::utc_now_iso()],
    )?;
    Ok(())
}

pub fn patch(conn: &mut Connection, patch: Value) -> Result<Settings, AppError> {
    let current = get_all(conn)?;
    let mut merged =
        serde_json::to_value(&current).map_err(|err| AppError::internal(err.to_string()))?;
    if let (Some(object), Some(patch_object)) = (merged.as_object_mut(), patch.as_object()) {
        for (key, value) in patch_object {
            if KEYS.contains(&key.as_str()) {
                object.insert(key.clone(), value.clone());
            }
        }
    }
    let next: Settings = serde_json::from_value(merged)
        .map_err(|err| AppError::new("bad_setting", "设置项不合法").with_detail(err.to_string()))?;
    validate(&next)?;
    let boundary_changed = next.day_boundary_minutes != current.day_boundary_minutes;
    let object = serde_json::to_value(&next).map_err(|err| AppError::internal(err.to_string()))?;
    let tx = conn.transaction()?;
    if boundary_changed {
        crate::db::journal::recompute_day_keys(&tx, next.day_boundary_minutes)?;
    }
    if let Some(map) = object.as_object() {
        for (key, value) in map {
            write_value(&tx, key, value)?;
        }
    }
    tx.commit()?;
    Ok(next)
}
