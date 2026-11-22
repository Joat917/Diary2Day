use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ThemeSlot {
    pub color: String,
    pub alpha: u8,
}

impl Default for ThemeSlot {
    fn default() -> Self {
        Self {
            color: "#FFFFFF".into(),
            alpha: 100,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CustomTheme {
    pub base: String,
    pub surface: ThemeSlot,
    pub card: ThemeSlot,
    pub inset: ThemeSlot,
    pub scrim: ThemeSlot,
    pub background: Option<String>,
}

impl Default for CustomTheme {
    fn default() -> Self {
        Self {
            base: "light".into(),
            surface: ThemeSlot {
                color: "#F6F7F6".into(),
                alpha: 100,
            },
            card: ThemeSlot {
                color: "#FFFFFF".into(),
                alpha: 100,
            },
            inset: ThemeSlot {
                color: "#EDEFEE".into(),
                alpha: 100,
            },
            scrim: ThemeSlot {
                color: "#111312".into(),
                alpha: 45,
            },
            background: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiFont {
    pub name: String,
    pub label: String,
}

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
    pub auto_edit: bool,
    pub export_width: u32,
    pub custom_theme: Option<CustomTheme>,
    pub ui_font: Option<UiFont>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // "system" 只是首次启动的哨兵值：前端按系统明暗落成具体主题后立即写回。
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
            auto_edit: true,
            export_width: 640,
            custom_theme: None,
            ui_font: None,
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
    "autoEdit",
    "exportWidth",
    "customTheme",
    "uiFont",
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
        auto_edit: read_field(conn, "autoEdit", default.auto_edit),
        export_width: read_field(conn, "exportWidth", default.export_width),
        custom_theme: read_field(conn, "customTheme", default.custom_theme),
        ui_font: read_field(conn, "uiFont", default.ui_font),
    })
}

fn hex_color_ok(color: &str) -> bool {
    let bytes = color.as_bytes();
    bytes.len() == 7
        && bytes[0] == b'#'
        && bytes[1..].iter().all(u8::is_ascii_hexdigit)
}

fn safe_stem(stem: &str) -> bool {
    !stem.is_empty()
        && stem
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn split_safe_name(name: &str) -> Option<(&str, &str)> {
    let (stem, ext) = name.rsplit_once('.')?;
    let ext_ok = (1..=5).contains(&ext.len())
        && ext
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    (safe_stem(stem) && ext_ok).then_some((stem, ext))
}

pub(crate) fn is_safe_image_name(name: &str) -> bool {
    split_safe_name(name).is_some_and(|(_, ext)| {
        matches!(ext, "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "avif")
    })
}

pub(crate) fn is_safe_font_name(name: &str) -> bool {
    split_safe_name(name)
        .is_some_and(|(_, ext)| matches!(ext, "ttf" | "otf" | "woff" | "woff2"))
}

fn validate_custom_theme(theme: &CustomTheme) -> Result<(), AppError> {
    if !matches!(theme.base.as_str(), "light" | "dark") {
        return Err(AppError::new("bad_setting", "自定义主题基色不合法"));
    }
    let slots = [
        ("surface", &theme.surface),
        ("card", &theme.card),
        ("inset", &theme.inset),
        ("scrim", &theme.scrim),
    ];
    for (name, slot) in slots {
        if !hex_color_ok(&slot.color) {
            return Err(AppError::new("bad_setting", "自定义主题色值不合法")
                .with_detail(format!("{name}: {}", slot.color)));
        }
        if slot.alpha > 100 {
            return Err(AppError::new("bad_setting", "自定义主题透明度超出范围")
                .with_detail(format!("{name}: {}", slot.alpha)));
        }
    }
    if let Some(background) = &theme.background {
        if !is_safe_image_name(background) {
            return Err(AppError::new("bad_setting", "背景图文件名不合法")
                .with_detail(background.clone()));
        }
    }
    Ok(())
}

fn validate(settings: &Settings) -> Result<(), AppError> {
    if !matches!(
        settings.theme.as_str(),
        "system" | "light" | "warm" | "mist" | "blossom" | "dark" | "midnight" | "mocha"
            | "forest" | "custom"
    ) {
        return Err(AppError::new("bad_setting", "未知主题"));
    }
    if let Some(custom) = &settings.custom_theme {
        validate_custom_theme(custom)?;
    }
    if settings.theme == "custom" && settings.custom_theme.is_none() {
        return Err(AppError::new("bad_setting", "自定义主题缺少配色"));
    }
    if let Some(font) = &settings.ui_font {
        if !is_safe_font_name(&font.name) {
            return Err(AppError::new("bad_setting", "字体文件名不合法")
                .with_detail(font.name.clone()));
        }
        if font.label.trim().chars().count() > 40 {
            return Err(AppError::new("bad_setting", "字体名称过长"));
        }
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
    if !(320..=1280).contains(&settings.export_width) {
        return Err(AppError::new("bad_setting", "导出图片宽度超出范围"));
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

#[cfg(test)]
mod tests {
    use super::{validate, CustomTheme, Settings, ThemeSlot, UiFont};

    fn with_custom(base: &str, surface: &str, alpha: u8) -> Settings {
        Settings {
            theme: "custom".into(),
            custom_theme: Some(CustomTheme {
                base: base.into(),
                surface: ThemeSlot {
                    color: surface.into(),
                    alpha,
                },
                ..CustomTheme::default()
            }),
            ..Settings::default()
        }
    }

    #[test]
    fn default_settings_are_valid() {
        assert!(validate(&Settings::default()).is_ok());
    }

    #[test]
    fn custom_theme_needs_palette() {
        let missing = Settings {
            theme: "custom".into(),
            ..Settings::default()
        };
        assert!(validate(&missing).is_err());
        assert!(validate(&with_custom("light", "#F6F7F6", 100)).is_ok());
    }

    #[test]
    fn rejects_bad_custom_theme_values() {
        assert!(validate(&with_custom("sepia", "#F6F7F6", 100)).is_err());
        assert!(validate(&with_custom("light", "F6F7F6", 100)).is_err());
        assert!(validate(&with_custom("light", "#F6F7", 100)).is_err());
        assert!(validate(&with_custom("light", "#GGGGGG", 100)).is_err());
        assert!(validate(&with_custom("light", "#F6F7F6", 101)).is_err());
        assert!(validate(&with_custom("dark", "#101211", 0)).is_ok());
    }

    #[test]
    fn rejects_unsafe_background_name() {
        let theme = CustomTheme {
            background: Some("../secret.jpg".into()),
            ..CustomTheme::default()
        };
        let settings = Settings {
            theme: "custom".into(),
            custom_theme: Some(theme),
            ..Settings::default()
        };
        assert!(validate(&settings).is_err());

        let theme = CustomTheme {
            background: Some("a1b2c3.jpg".into()),
            ..CustomTheme::default()
        };
        let settings = Settings {
            theme: "custom".into(),
            custom_theme: Some(theme),
            ..Settings::default()
        };
        assert!(validate(&settings).is_ok());
    }

    #[test]
    fn validates_ui_font() {
        let ok = Settings {
            ui_font: Some(UiFont {
                name: "f3c9a1.woff2".into(),
                label: "霞鹜文楷".into(),
            }),
            ..Settings::default()
        };
        assert!(validate(&ok).is_ok());

        let bad_ext = Settings {
            ui_font: Some(UiFont {
                name: "f3c9a1.ttc".into(),
                label: "集合".into(),
            }),
            ..Settings::default()
        };
        assert!(validate(&bad_ext).is_err());

        let traversal = Settings {
            ui_font: Some(UiFont {
                name: "../evil.ttf".into(),
                label: "evil".into(),
            }),
            ..Settings::default()
        };
        assert!(validate(&traversal).is_err());

        let long_label = Settings {
            ui_font: Some(UiFont {
                name: "f3c9a1.ttf".into(),
                label: "字".repeat(41),
            }),
            ..Settings::default()
        };
        assert!(validate(&long_label).is_err());
    }
}
