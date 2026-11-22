use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::error::AppError;

const LEGACY_THEME_ID: &str = "legacy-theme-1";

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomTheme {
    pub id: String,
    pub surface: ThemeSlot,
    pub card: ThemeSlot,
    pub inset: ThemeSlot,
    pub scrim: ThemeSlot,
    pub on_surface: String,
    pub on_surface_muted: String,
    pub primary: String,
    pub hairline: ThemeSlot,
    pub sample_title: Option<String>,
    pub sample_text: Option<String>,
    pub background: Option<String>,
    pub fonts: Vec<UiFont>,
}

const MAX_THEME_FONTS: usize = 3;

// 旧数据只有浅/深 base：缺四项文字色时按它补预设种子，读入后 base 不再存在。
#[derive(Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct CustomThemeWire {
    id: String,
    base: Option<String>,
    surface: ThemeSlot,
    card: ThemeSlot,
    inset: ThemeSlot,
    scrim: ThemeSlot,
    on_surface: Option<String>,
    on_surface_muted: Option<String>,
    primary: Option<String>,
    hairline: Option<HairlineWire>,
    sample_title: Option<String>,
    sample_text: Option<String>,
    background: Option<String>,
    // 上一版每只主题只有一个 `font` 对象：缺 `fonts` 时按一项读入。
    font: Option<UiFont>,
    fonts: Vec<UiFont>,
}

impl Default for CustomThemeWire {
    fn default() -> Self {
        Self {
            id: String::new(),
            base: None,
            surface: ThemeSlot::default(),
            card: ThemeSlot::default(),
            inset: ThemeSlot::default(),
            scrim: ThemeSlot::default(),
            on_surface: None,
            on_surface_muted: None,
            primary: None,
            hairline: None,
            sample_title: None,
            sample_text: None,
            background: None,
            font: None,
            fonts: Vec::new(),
        }
    }
}

// 描边早期只有色值字符串，读入时补成不透明的主题槽。
#[derive(Deserialize)]
#[serde(untagged)]
enum HairlineWire {
    Slot(ThemeSlot),
    Legacy(String),
}

impl From<HairlineWire> for ThemeSlot {
    fn from(wire: HairlineWire) -> Self {
        match wire {
            HairlineWire::Slot(slot) => slot,
            HairlineWire::Legacy(color) => Self { color, alpha: 100 },
        }
    }
}

fn seed_text_colors(dark: bool) -> (&'static str, &'static str, &'static str, &'static str) {
    if dark {
        ("#ECEEED", "#9BA29F", "#5FC9B4", "#2A2F2D")
    } else {
        ("#17181A", "#5F6467", "#17695C", "#E4E7E6")
    }
}

impl<'de> Deserialize<'de> for CustomTheme {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = CustomThemeWire::deserialize(deserializer)?;
        let (text, muted, primary, hairline) = seed_text_colors(wire.base.as_deref() == Some("dark"));
        Ok(Self {
            id: wire.id,
            surface: wire.surface,
            card: wire.card,
            inset: wire.inset,
            scrim: wire.scrim,
            on_surface: wire.on_surface.unwrap_or_else(|| text.into()),
            on_surface_muted: wire.on_surface_muted.unwrap_or_else(|| muted.into()),
            primary: wire.primary.unwrap_or_else(|| primary.into()),
            hairline: wire.hairline.map(ThemeSlot::from).unwrap_or_else(|| ThemeSlot {
                color: hairline.into(),
                alpha: 100,
            }),
            sample_title: wire.sample_title,
            sample_text: wire.sample_text,
            background: wire.background,
            fonts: if wire.fonts.is_empty() {
                wire.font.into_iter().collect()
            } else {
                wire.fonts
            },
        })
    }
}

impl Default for CustomTheme {
    fn default() -> Self {
        let (on_surface, on_surface_muted, primary, hairline) = seed_text_colors(false);
        Self {
            id: String::new(),
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
            on_surface: on_surface.into(),
            on_surface_muted: on_surface_muted.into(),
            primary: primary.into(),
            hairline: ThemeSlot {
                color: hairline.into(),
                alpha: 100,
            },
            sample_title: None,
            sample_text: None,
            background: None,
            fonts: Vec::new(),
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
    pub custom_themes: Vec<CustomTheme>,
    pub custom_theme_id: Option<String>,
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
            custom_themes: Vec::new(),
            custom_theme_id: None,
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
    "customThemes",
    "customThemeId",
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

fn read_legacy_theme(conn: &Connection) -> Option<CustomTheme> {
    let value = read_value(conn, "customTheme").ok().flatten()?;
    let mut theme: CustomTheme = serde_json::from_value(value).ok()?;
    theme.id = LEGACY_THEME_ID.into();
    let font: Option<UiFont> = read_value(conn, "uiFont")
        .ok()
        .flatten()
        .and_then(|value| serde_json::from_value(value).ok());
    theme.fonts = font.into_iter().collect();
    Some(theme)
}

pub fn get_all(conn: &Connection) -> Result<Settings, AppError> {
    let default = Settings::default();
    let theme = read_field(conn, "theme", default.theme);
    let mut custom_themes: Vec<CustomTheme> =
        read_field(conn, "customThemes", default.custom_themes);
    if custom_themes.is_empty() {
        if let Some(legacy) = read_legacy_theme(conn) {
            custom_themes.push(legacy);
        }
    }
    let mut custom_theme_id: Option<String> =
        read_field(conn, "customThemeId", default.custom_theme_id);
    if custom_theme_id.is_none() && theme == "custom" {
        custom_theme_id = custom_themes.first().map(|custom| custom.id.clone());
    }
    Ok(Settings {
        theme,
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
        custom_themes,
        custom_theme_id,
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

fn safe_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn validate_font(font: &UiFont) -> Result<(), AppError> {
    if !is_safe_font_name(&font.name) {
        return Err(AppError::new("bad_setting", "字体文件名不合法").with_detail(font.name.clone()));
    }
    if font.label.trim().chars().count() > 40 {
        return Err(AppError::new("bad_setting", "字体名称过长"));
    }
    Ok(())
}

fn validate_sample_text(
    label: &str,
    value: Option<&String>,
    max: usize,
) -> Result<(), AppError> {
    let Some(text) = value else {
        return Ok(());
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(AppError::new("bad_setting", format!("自定义主题{label}不能为空")));
    }
    if trimmed.chars().count() > max {
        return Err(AppError::new("bad_setting", format!("自定义主题{label}过长"))
            .with_detail(format!("{} 字（上限 {max}）", trimmed.chars().count())));
    }
    Ok(())
}

fn validate_custom_theme(theme: &CustomTheme) -> Result<(), AppError> {
    if !safe_id(&theme.id) {
        return Err(AppError::new("bad_setting", "自定义主题 id 不合法")
            .with_detail(theme.id.clone()));
    }
    for (name, color) in [
        ("onSurface", &theme.on_surface),
        ("onSurfaceMuted", &theme.on_surface_muted),
        ("primary", &theme.primary),
    ] {
        if !hex_color_ok(color) {
            return Err(AppError::new("bad_setting", "自定义主题文字颜色不合法")
                .with_detail(format!("{name}: {color}")));
        }
    }
    validate_sample_text("标题", theme.sample_title.as_ref(), 40)?;
    validate_sample_text("说明文字", theme.sample_text.as_ref(), 120)?;
    let slots = [
        ("surface", &theme.surface),
        ("card", &theme.card),
        ("inset", &theme.inset),
        ("scrim", &theme.scrim),
        ("hairline", &theme.hairline),
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
    if theme.fonts.len() > MAX_THEME_FONTS {
        return Err(AppError::new("bad_setting", "自定义主题字体过多")
            .with_detail(format!("{} 个（上限 {MAX_THEME_FONTS}）", theme.fonts.len())));
    }
    for (index, font) in theme.fonts.iter().enumerate() {
        validate_font(font)?;
        if theme.fonts[..index]
            .iter()
            .any(|other| other.name == font.name)
        {
            return Err(AppError::new("bad_setting", "自定义主题字体重复")
                .with_detail(font.name.clone()));
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
    for (index, custom) in settings.custom_themes.iter().enumerate() {
        validate_custom_theme(custom)?;
        if settings.custom_themes[..index]
            .iter()
            .any(|other| other.id == custom.id)
        {
            return Err(AppError::new("bad_setting", "自定义主题 id 重复")
                .with_detail(custom.id.clone()));
        }
    }
    if settings.theme == "custom" {
        let id = settings
            .custom_theme_id
            .as_deref()
            .ok_or_else(|| AppError::new("bad_setting", "自定义主题缺少当前项"))?;
        if !settings.custom_themes.iter().any(|custom| custom.id == id) {
            return Err(AppError::new("bad_setting", "当前自定义主题不存在").with_detail(id));
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
    tx.execute("DELETE FROM settings WHERE key IN ('customTheme', 'uiFont')", [])?;
    tx.commit()?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::{CustomTheme, Settings, ThemeSlot, UiFont, get_all, patch, validate};
    use rusqlite::{Connection, params};
    use serde_json::json;

    fn with_custom(surface: &str, alpha: u8) -> Settings {
        Settings {
            theme: "custom".into(),
            custom_themes: vec![CustomTheme {
                id: "theme-1".into(),
                surface: ThemeSlot {
                    color: surface.into(),
                    alpha,
                },
                ..CustomTheme::default()
            }],
            custom_theme_id: Some("theme-1".into()),
            ..Settings::default()
        }
    }

    fn with_fonts(fonts: Vec<UiFont>) -> Settings {
        let mut settings = with_custom("#F6F7F6", 100);
        settings.custom_themes[0].fonts = fonts;
        settings
    }

    fn font(name: &str, label: &str) -> UiFont {
        UiFont {
            name: name.into(),
            label: label.into(),
        }
    }

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open memory db");
        conn.execute_batch(
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL);",
        )
        .expect("create settings table");
        conn
    }

    fn insert_setting(conn: &Connection, key: &str, value: serde_json::Value) {
        conn.execute(
            "INSERT INTO settings(key, value, updated_at) VALUES(?1, ?2, ?3)",
            params![key, value.to_string(), "2026-01-01T00:00:00Z"],
        )
        .expect("insert setting");
    }

    #[test]
    fn default_settings_are_valid() {
        assert!(validate(&Settings::default()).is_ok());
    }

    #[test]
    fn custom_theme_needs_an_existing_active_item() {
        let missing = Settings {
            theme: "custom".into(),
            ..Settings::default()
        };
        assert!(validate(&missing).is_err());

        let no_id = Settings {
            theme: "custom".into(),
            custom_themes: vec![CustomTheme {
                id: "theme-1".into(),
                ..CustomTheme::default()
            }],
            custom_theme_id: None,
            ..Settings::default()
        };
        assert!(validate(&no_id).is_err());

        let dangling = Settings {
            theme: "custom".into(),
            custom_themes: vec![CustomTheme {
                id: "theme-1".into(),
                ..CustomTheme::default()
            }],
            custom_theme_id: Some("theme-2".into()),
            ..Settings::default()
        };
        assert!(validate(&dangling).is_err());

        assert!(validate(&with_custom("#F6F7F6", 100)).is_ok());
    }

    #[test]
    fn rejects_duplicate_and_bad_theme_ids() {
        let mut duplicate = with_custom("#F6F7F6", 100);
        duplicate
            .custom_themes
            .push(duplicate.custom_themes[0].clone());
        assert!(validate(&duplicate).is_err());

        for id in ["", "bad id", "../evil", &"a".repeat(65)] {
            let mut settings = with_custom("#F6F7F6", 100);
            settings.custom_themes[0].id = id.into();
            settings.custom_theme_id = Some(id.into());
            assert!(validate(&settings).is_err(), "id should be rejected: {id}");
        }

        let mut ok = with_custom("#F6F7F6", 100);
        ok.custom_themes[0].id = "A_9-z".into();
        ok.custom_theme_id = Some("A_9-z".into());
        assert!(validate(&ok).is_ok());
    }

    #[test]
    fn rejects_bad_custom_theme_values() {
        assert!(validate(&with_custom("F6F7F6", 100)).is_err());
        assert!(validate(&with_custom("#F6F7", 100)).is_err());
        assert!(validate(&with_custom("#GGGGGG", 100)).is_err());
        assert!(validate(&with_custom("#F6F7F6", 101)).is_err());
        assert!(validate(&with_custom("#101211", 0)).is_ok());

        for (field, value) in [
            ("onSurface", "17181A"),
            ("onSurfaceMuted", "#56"),
            ("primary", "#XYZXYZ"),
        ] {
            let mut settings = with_custom("#F6F7F6", 100);
            match field {
                "onSurface" => settings.custom_themes[0].on_surface = value.into(),
                "onSurfaceMuted" => settings.custom_themes[0].on_surface_muted = value.into(),
                _ => settings.custom_themes[0].primary = value.into(),
            }
            assert!(
                validate(&settings).is_err(),
                "text color should be rejected: {field}"
            );
        }
    }

    #[test]
    fn validates_hairline_slot() {
        let mut settings = with_custom("#F6F7F6", 100);
        settings.custom_themes[0].hairline = ThemeSlot {
            color: "#E4E7E6".into(),
            alpha: 35,
        };
        assert!(validate(&settings).is_ok());

        settings.custom_themes[0].hairline.alpha = 0;
        assert!(validate(&settings).is_ok());

        settings.custom_themes[0].hairline.alpha = 101;
        assert!(validate(&settings).is_err());

        settings.custom_themes[0].hairline = ThemeSlot {
            color: "E4E7E6".into(),
            alpha: 50,
        };
        assert!(validate(&settings).is_err());

        settings.custom_themes[0].hairline = ThemeSlot {
            color: String::new(),
            alpha: 50,
        };
        assert!(validate(&settings).is_err());
    }

    #[test]
    fn reads_legacy_hairline_string_as_opaque() {
        let mut conn = test_db();
        insert_setting(&conn, "theme", json!("custom"));
        insert_setting(
            &conn,
            "customThemes",
            json!([{
                "id": "theme-2",
                "surface": { "color": "#101211", "alpha": 100 },
                "card": { "color": "#181A19", "alpha": 100 },
                "inset": { "color": "#0C0D0D", "alpha": 100 },
                "scrim": { "color": "#000000", "alpha": 60 },
                "onSurface": "#ECEEED",
                "onSurfaceMuted": "#9BA29F",
                "primary": "#5FC9B4",
                "hairline": "#2A2F2D"
            }]),
        );

        let settings = get_all(&conn).expect("read settings");
        assert_eq!(
            settings.custom_themes[0].hairline,
            ThemeSlot {
                color: "#2A2F2D".into(),
                alpha: 100,
            }
        );

        let stored = serde_json::to_value(&settings.custom_themes[0]).expect("serialize theme");
        assert_eq!(
            stored.get("hairline"),
            Some(&json!({ "color": "#2A2F2D", "alpha": 100 }))
        );

        let next = patch(&mut conn, json!({ "language": "en" })).expect("patch settings");
        assert_eq!(next.custom_themes[0].hairline.alpha, 100);
    }

    #[test]
    fn preserves_hairline_alpha() {
        let mut settings = with_custom("#F6F7F6", 100);
        settings.custom_themes[0].hairline = ThemeSlot {
            color: "#E4E7E6".into(),
            alpha: 42,
        };
        let mut conn = test_db();
        let next = patch(
            &mut conn,
            serde_json::to_value(&settings).expect("serialize settings"),
        )
        .expect("patch settings");
        assert_eq!(next.custom_themes[0].hairline.color, "#E4E7E6");
        assert_eq!(next.custom_themes[0].hairline.alpha, 42);
    }

    #[test]
    fn rejects_unsafe_background_name() {
        let mut settings = with_custom("#F6F7F6", 100);
        settings.custom_themes[0].background = Some("../secret.jpg".into());
        assert!(validate(&settings).is_err());

        settings.custom_themes[0].background = Some("a1b2c3.jpg".into());
        assert!(validate(&settings).is_ok());
    }

    #[test]
    fn validates_font_per_theme() {
        assert!(
            validate(&with_fonts(vec![
                font("f3c9a1.woff2", "霞鹜文楷"),
                font("b7d20c.ttf", "Inter"),
            ]))
            .is_ok()
        );

        let long_label = "字".repeat(41);
        for (name, label) in [
            ("f3c9a1.ttc", "集合"),
            ("../evil.ttf", "evil"),
            ("f3c9a1.ttf", long_label.as_str()),
        ] {
            assert!(
                validate(&with_fonts(vec![font(name, label)])).is_err(),
                "font should be rejected: {name}"
            );
        }

        let too_many = (0..4)
            .map(|index| font(&format!("f{index}.ttf"), "字体"))
            .collect();
        assert!(validate(&with_fonts(too_many)).is_err());

        assert!(
            validate(&with_fonts(vec![
                font("f3c9a1.ttf", "楷体"),
                font("f3c9a1.ttf", "重复"),
            ]))
            .is_err()
        );
    }

    #[test]
    fn migrates_single_font_to_list() {
        let conn = test_db();
        insert_setting(&conn, "theme", json!("custom"));
        insert_setting(
            &conn,
            "customThemes",
            json!([{
                "id": "theme-2",
                "surface": { "color": "#101211", "alpha": 100 },
                "card": { "color": "#181A19", "alpha": 100 },
                "inset": { "color": "#0C0D0D", "alpha": 100 },
                "scrim": { "color": "#000000", "alpha": 60 },
                "font": { "name": "f3c9a1.ttf", "label": "霞鹜文楷" }
            }]),
        );

        let settings = get_all(&conn).expect("read settings");
        assert_eq!(settings.custom_themes[0].fonts.len(), 1);
        assert_eq!(settings.custom_themes[0].fonts[0].label, "霞鹜文楷");

        let stored = serde_json::to_value(&settings.custom_themes[0]).expect("serialize theme");
        assert!(stored.get("font").is_none());
        assert_eq!(
            stored.get("fonts"),
            Some(&json!([{ "name": "f3c9a1.ttf", "label": "霞鹜文楷" }]))
        );
    }

    #[test]
    fn validates_sample_texts() {
        let mut blank = with_custom("#F6F7F6", 100);
        blank.custom_themes[0].sample_title = Some("   ".into());
        assert!(validate(&blank).is_err());

        let mut long_title = with_custom("#F6F7F6", 100);
        long_title.custom_themes[0].sample_title = Some("字".repeat(41));
        assert!(validate(&long_title).is_err());

        let mut long_text = with_custom("#F6F7F6", 100);
        long_text.custom_themes[0].sample_text = Some("字".repeat(121));
        assert!(validate(&long_text).is_err());

        let mut ok = with_custom("#F6F7F6", 100);
        ok.custom_themes[0].sample_title = Some("我的主题".into());
        ok.custom_themes[0].sample_text = Some("长按编辑该主题".into());
        assert!(validate(&ok).is_ok());

        let stored = serde_json::to_value(&ok.custom_themes[0]).expect("serialize theme");
        assert_eq!(
            stored.get("sampleTitle").and_then(|value| value.as_str()),
            Some("我的主题")
        );
        assert_eq!(
            stored.get("sampleText").and_then(|value| value.as_str()),
            Some("长按编辑该主题")
        );

        let plain = serde_json::to_value(CustomTheme::default()).expect("serialize default theme");
        assert!(plain["sampleTitle"].is_null());
        assert!(plain["sampleText"].is_null());
    }

    #[test]
    fn legacy_base_only_seeds_missing_text_colors() {
        let conn = test_db();
        insert_setting(&conn, "theme", json!("custom"));
        insert_setting(
            &conn,
            "customThemes",
            json!([{
                "id": "theme-2",
                "base": "dark",
                "primary": "#ABCDEF",
                "surface": { "color": "#101211", "alpha": 100 },
                "card": { "color": "#181A19", "alpha": 100 },
                "inset": { "color": "#0C0D0D", "alpha": 100 },
                "scrim": { "color": "#000000", "alpha": 60 }
            }]),
        );

        let settings = get_all(&conn).expect("read settings");
        let theme = &settings.custom_themes[0];
        assert_eq!(theme.on_surface, "#ECEEED");
        assert_eq!(theme.on_surface_muted, "#9BA29F");
        assert_eq!(theme.primary, "#ABCDEF");
        assert_eq!(theme.hairline.color, "#2A2F2D");
        assert_eq!(theme.hairline.alpha, 100);

        let stored = serde_json::to_value(theme).expect("serialize theme");
        assert!(stored.get("base").is_none());
        assert_eq!(
            stored.get("onSurface").and_then(|value| value.as_str()),
            Some("#ECEEED")
        );
    }

    #[test]
    fn migrates_legacy_single_theme() {
        let conn = test_db();
        insert_setting(&conn, "theme", json!("custom"));
        insert_setting(
            &conn,
            "customTheme",
            json!({
                "base": "dark",
                "surface": { "color": "#101211", "alpha": 90 },
                "card": { "color": "#181A19", "alpha": 80 },
                "inset": { "color": "#0C0D0D", "alpha": 100 },
                "scrim": { "color": "#000000", "alpha": 60 },
                "background": "a1b2c3.jpg"
            }),
        );
        insert_setting(
            &conn,
            "uiFont",
            json!({ "name": "f3c9a1.ttf", "label": "霞鹜文楷" }),
        );

        let migrated = get_all(&conn).expect("read settings");
        assert_eq!(migrated.custom_themes.len(), 1);
        assert_eq!(migrated.custom_themes[0].id, "legacy-theme-1");
        assert_eq!(migrated.custom_themes[0].on_surface, "#ECEEED");
        assert_eq!(migrated.custom_themes[0].on_surface_muted, "#9BA29F");
        assert_eq!(migrated.custom_themes[0].primary, "#5FC9B4");
        assert_eq!(migrated.custom_themes[0].hairline.color, "#2A2F2D");
        assert_eq!(migrated.custom_themes[0].hairline.alpha, 100);
        assert_eq!(migrated.custom_themes[0].fonts.len(), 1);
        assert_eq!(migrated.custom_themes[0].fonts[0].label, "霞鹜文楷");
        assert_eq!(migrated.custom_theme_id.as_deref(), Some("legacy-theme-1"));

        let mut conn = conn;
        let next = patch(&mut conn, json!({ "language": "en" })).expect("patch settings");
        assert_eq!(next.custom_themes.len(), 1);
        let legacy_left: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE key IN ('customTheme', 'uiFont')",
                [],
                |row| row.get(0),
            )
            .expect("count legacy keys");
        assert_eq!(legacy_left, 0);
    }
}
