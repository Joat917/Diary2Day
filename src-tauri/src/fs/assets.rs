use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use rusqlite::params;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::db::journal;
use crate::error::AppError;
use crate::state::AppState;

const READ_BUFFER: usize = 64 * 1024;
const EXPORT_VIEW_DIR: &str = ".export";
// 副本最多按卡片宽度的 2 倍像素密度准备。
const EXPORT_VIEW_SCALE: u32 = 2;
// 高度不设限，长图按比例保留。Android WebView 会整张拒绝解码大位图
// （36MP 实测失败），限宽管不住长图，所以只在那里再用总像素数兜底；
// 桌面 WebView 能解码，只按设置宽度限宽。
#[cfg(target_os = "android")]
const EXPORT_VIEW_MAX_PIXELS: Option<u64> = Some(8_388_608);
#[cfg(not(target_os = "android"))]
const EXPORT_VIEW_MAX_PIXELS: Option<u64> = None;

#[cfg(not(target_os = "android"))]
pub const MEDIA_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "bmp", "avif", "mp3", "wav", "ogg", "m4a", "aac", "flac",
    "mp4", "webm", "mov", "mkv",
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDto {
    pub name: String,
    pub kind: String,
    pub ext: String,
    pub size: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

pub fn kind_for(ext: &str) -> Option<&'static str> {
    match ext {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "avif" => Some("image"),
        "mp3" | "wav" | "ogg" | "m4a" | "aac" | "flac" => Some("audio"),
        "mp4" | "webm" | "mov" | "mkv" => Some("video"),
        _ => None,
    }
}

fn sniff_ext(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("jpg");
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("png");
    }
    if bytes.starts_with(b"GIF8") {
        return Some("gif");
    }
    if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("webp");
    }
    if bytes.len() > 12 && &bytes[4..8] == b"ftyp" {
        return Some("mp4");
    }
    None
}

fn ext_of(name: &str) -> Option<String> {
    let ext = name.rsplit_once('.')?.1.to_ascii_lowercase();
    let valid = (1..=5).contains(&ext.len())
        && ext
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    valid.then_some(ext)
}

pub fn is_media_name(name: &str) -> bool {
    ext_of(name).as_deref().and_then(kind_for).is_some()
}


fn write_thumbnail(assets_dir: &Path, name: &str, path: &Path) {
    let stem = name.rsplit_once('.').map(|(value, _)| value).unwrap_or(name);
    let thumbs = assets_dir.join(".thumbs");
    if fs::create_dir_all(&thumbs).is_err() {
        return;
    }
    let Ok(image) = image::open(path) else {
        return;
    };
    let thumb = image.thumbnail(512, 512);
    let webp = thumbs.join(format!("{stem}.webp"));
    if thumb
        .save_with_format(&webp, image::ImageFormat::WebP)
        .is_ok()
    {
        return;
    }
    let _ = thumb.save_with_format(thumbs.join(format!("{stem}.jpg")), image::ImageFormat::Jpeg);
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportViewDto {
    pub name: String,
    pub view: String,
}

// 大图不在 WebView 里解码，改由这里生成限宽副本（Android 另限总像素），导出时替换 img 的 src。
// 高按比例走；已经是小图就不生成，前端继续用原图。
fn export_view_size(
    max_width: u32,
    max_pixels: Option<u64>,
    width: u32,
    height: u32,
) -> Option<(u32, u32)> {
    let pixels = u64::from(width) * u64::from(height);
    if width <= max_width && max_pixels.map_or(true, |budget| pixels <= budget) {
        return None;
    }
    let width_scale = f64::from(max_width) / f64::from(width);
    let scale = match max_pixels {
        Some(budget) => width_scale
            .min((budget as f64 / pixels as f64).sqrt())
            .min(1.0),
        None => width_scale.min(1.0),
    };
    Some((
        ((f64::from(width) * scale).round() as u32).max(1),
        ((f64::from(height) * scale).round() as u32).max(1),
    ))
}

// 相机 JPEG 的方向在 EXIF 里，WebView 会照它显示，副本也要先拍平；
// 90°/270° 会交换宽高，尺寸决策必须按拍平后的宽高算。
fn export_view(name: &str, path: &Path, dir: &Path, max_width: u32) -> Option<String> {
    use image::metadata::Orientation;
    use image::{DynamicImage, ImageDecoder, ImageReader};

    let mut decoder = ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .into_decoder()
        .ok()?;
    let (raw_width, raw_height) = decoder.dimensions();
    let orientation = decoder.orientation().ok()?;
    let (width, height) = match orientation {
        Orientation::Rotate90
        | Orientation::Rotate270
        | Orientation::Rotate90FlipH
        | Orientation::Rotate270FlipH => (raw_height, raw_width),
        _ => (raw_width, raw_height),
    };
    let (new_width, new_height) =
        export_view_size(max_width, EXPORT_VIEW_MAX_PIXELS, width, height)?;

    let stem = name.rsplit_once('.').map(|(value, _)| value).unwrap_or(name);
    let view = format!("{stem}.webp");
    let view_path = dir.join(&view);
    // 导出宽度可调：已有副本与当前目标宽度不符（调大或调小）就按新尺寸重建。
    let current_width = image::image_dimensions(&view_path)
        .map(|(existing, _)| existing)
        .ok();
    if current_width != Some(new_width) {
        if fs::create_dir_all(dir).is_err() {
            return None;
        }
        let mut image = DynamicImage::from_decoder(decoder).ok()?;
        image.apply_orientation(orientation);
        let resized = image.resize_exact(
            new_width,
            new_height,
            image::imageops::FilterType::Lanczos3,
        );
        if resized
            .save_with_format(&view_path, image::ImageFormat::WebP)
            .is_err()
        {
            let _ = fs::remove_file(&view_path);
            return None;
        }
    }
    Some(view)
}

pub fn export_views(
    state: &AppState,
    names: Vec<String>,
    width: u32,
) -> Result<Vec<ExportViewDto>, AppError> {
    let dir = state.paths.assets_dir.join(EXPORT_VIEW_DIR);
    let max_width = width.clamp(320, 1280).saturating_mul(EXPORT_VIEW_SCALE);
    let mut out = Vec::new();
    for name in names {
        if checked_kind(&name) != Some("image") {
            continue;
        }
        let path = state.paths.assets_dir.join(&name);
        if let Some(view) = export_view(&name, &path, &dir, max_width) {
            out.push(ExportViewDto { name, view });
        }
    }
    Ok(out)
}

pub fn import_reader(
    state: &AppState,
    orig_name: &str,
    reader: &mut impl Read,
) -> Result<AssetDto, AppError> {
    let mut buffer = vec![0u8; READ_BUFFER];
    let read = reader.read(&mut buffer)?;
    if read == 0 {
        return Err(AppError::new("bad_asset", "附件内容为空"));
    }
    let ext = ext_of(orig_name)
        .filter(|ext| kind_for(ext).is_some())
        .or_else(|| sniff_ext(&buffer[..read]).map(str::to_string))
        .ok_or_else(|| AppError::new("bad_asset", "无法识别文件类型"))?;
    let kind = kind_for(&ext)
        .ok_or_else(|| AppError::new("bad_asset", "不支持的文件类型"))?
        .to_string();

    let stem = uuid::Uuid::now_v7().simple().to_string();
    let name = format!("{stem}.{ext}");
    let path: PathBuf = state.paths.assets_dir.join(&name);

    let mut handle = fs::File::create(&path)?;
    let mut hasher = Sha256::new();
    let mut size: u64 = 0;
    let written = (|| -> std::io::Result<()> {
        handle.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
        size += read as u64;
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            handle.write_all(&buffer[..read])?;
            hasher.update(&buffer[..read]);
            size += read as u64;
        }
        handle.flush()
    })();
    drop(handle);
    if let Err(err) = written {
        let _ = fs::remove_file(&path);
        return Err(err.into());
    }

    let (width, height) = if kind == "image" {
        image::image_dimensions(&path)
            .map(|(w, h)| (Some(w), Some(h)))
            .unwrap_or((None, None))
    } else {
        (None, None)
    };
    if kind == "image" {
        write_thumbnail(&state.paths.assets_dir, &name, &path);
    }

    let hash = hasher.finalize();
    state.with_db(|conn| {
        conn.execute(
            "INSERT INTO asset(name, kind, ext, size, width, height, duration_ms, hash, \
             orig_name, created_at) VALUES(?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?9)",
            params![
                name,
                kind,
                ext,
                size as i64,
                width,
                height,
                hash.as_slice(),
                orig_name,
                crate::domain::utc_now_iso()
            ],
        )?;
        Ok(())
    })?;

    Ok(AssetDto {
        name,
        kind,
        ext,
        size,
        width,
        height,
    })
}

#[cfg(not(target_os = "android"))]
pub fn import_bytes(state: &AppState, orig_name: &str, bytes: &[u8]) -> Result<AssetDto, AppError> {
    let mut cursor = bytes;
    import_reader(state, orig_name, &mut cursor)
}

pub fn import_path(state: &AppState, path: &Path) -> Result<Option<AssetDto>, AppError> {
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    if !is_media_name(&name) {
        return Ok(None);
    }
    if !fs::metadata(path)?.is_file() {
        return Ok(None);
    }
    let mut input = fs::File::open(path)?;
    import_reader(state, &name, &mut input).map(Some)
}

pub fn scan_asset_refs(content: &str) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = Vec::new();
    for kind in ["image", "audio", "video"] {
        let prefix = format!("ddasset://{kind}/");
        let mut rest = content;
        while let Some(pos) = rest.find(&prefix) {
            let after = &rest[pos + prefix.len()..];
            let end = after
                .find(|c: char| {
                    !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
                })
                .unwrap_or(after.len());
            if end > 0 {
                let name = after[..end].to_string();
                if !found.iter().any(|(_, existing)| existing == &name) {
                    found.push((kind.to_string(), name));
                }
            }
            rest = &after[end..];
        }
    }
    found
}

fn checked_kind(name: &str) -> Option<&'static str> {
    let (stem, ext) = name.rsplit_once('.')?;
    if stem.is_empty()
        || !stem
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    if ext.is_empty()
        || ext.len() > 5
        || !ext
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    {
        return None;
    }
    kind_for(ext)
}

fn thumb_shared(assets_dir: &Path, stem: &str, own: &str) -> bool {
    let Ok(entries) = fs::read_dir(assets_dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == own {
            return false;
        }
        name.rsplit_once('.')
            .map(|(other_stem, ext)| other_stem == stem && kind_for(ext).is_some())
            .unwrap_or(false)
    })
}

fn remove_unused(state: &AppState, name: &str) -> Result<bool, AppError> {
    let path = state.paths.assets_dir.join(name);
    let removed = match fs::remove_file(&path) {
        Ok(()) => true,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => false,
        Err(err) => return Err(AppError::io(err.to_string())),
    };
    let stem = name.rsplit_once('.').map(|(value, _)| value).unwrap_or(name);
    if !thumb_shared(&state.paths.assets_dir, stem, name) {
        let thumbs = state.paths.assets_dir.join(".thumbs");
        let _ = fs::remove_file(thumbs.join(format!("{stem}.webp")));
        let _ = fs::remove_file(thumbs.join(format!("{stem}.jpg")));
        let _ = fs::remove_file(
            state
                .paths
                .assets_dir
                .join(EXPORT_VIEW_DIR)
                .join(format!("{stem}.webp")),
        );
    }
    state.with_db(|conn| {
        conn.execute("DELETE FROM asset WHERE name = ?1", [name])?;
        Ok(())
    })?;
    Ok(removed)
}

/// Deletes the given assets. The caller decides what is unused from the
/// document content (loaded assets plus newly imported ones, minus the final
/// references), so no cross-document scan happens here.
pub fn gc_unused(state: &AppState, names: Vec<String>) -> Result<usize, AppError> {
    let mut removed = 0;
    for name in names {
        if checked_kind(&name).is_none() {
            continue;
        }
        if remove_unused(state, &name)? {
            removed += 1;
        }
    }
    Ok(removed)
}

/// Deletes assets referenced by permanently removed entries unless a remaining
/// entry still mentions them.
pub fn remove_orphans(state: &AppState, contents: &[String]) -> Result<(), AppError> {
    let mut names: Vec<String> = Vec::new();
    for (_, name) in contents
        .iter()
        .flat_map(|content| scan_asset_refs(content.as_str()))
    {
        if checked_kind(&name).is_some() && !names.contains(&name) {
            names.push(name);
        }
    }
    for name in names {
        let still_used = state.with_db(|conn| {
            Ok::<bool, AppError>(conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM journal WHERE instr(content, ?1) > 0)",
                [&name],
                |row| row.get(0),
            )?)
        })?;
        if !still_used {
            remove_unused(state, &name)?;
        }
    }
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupReport {
    pub removed: usize,
    pub freed: u64,
}

/// Deletes every media file in `assets_dir` that no journal content references.
/// Runs only on explicit request; the editing session's own garbage collection
/// stays the default.
pub fn clean_unused(state: &AppState) -> Result<CleanupReport, AppError> {
    let contents = state.with_db(|conn| journal::all_contents(conn))?;
    let used: HashSet<String> = contents
        .iter()
        .flat_map(|content| scan_asset_refs(content.as_str()))
        .map(|(_, name)| name)
        .collect();

    let mut candidates: Vec<(String, u64)> = Vec::new();
    for entry in fs::read_dir(&state.paths.assets_dir)?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if checked_kind(&name).is_none() || used.contains(&name) {
            continue;
        }
        let size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        candidates.push((name, size));
    }

    let mut removed = 0;
    let mut freed = 0;
    for (name, size) in candidates {
        if remove_unused(state, &name)? {
            removed += 1;
            freed += size;
        }
    }
    Ok(CleanupReport { removed, freed })
}

#[cfg(test)]
mod tests {
    use super::export_view_size;

    const ANDROID_BUDGET: Option<u64> = Some(8_388_608);

    #[test]
    fn small_images_are_used_as_is() {
        assert_eq!(export_view_size(1280, ANDROID_BUDGET, 1200, 900), None);
        assert_eq!(export_view_size(2000, ANDROID_BUDGET, 1920, 1080), None);
    }

    #[test]
    fn wide_images_are_limited_by_width_and_keep_aspect() {
        let (width, height) =
            export_view_size(1280, ANDROID_BUDGET, 4223, 8598).expect("需要导出副本");
        assert_eq!(width, 1280);
        assert_eq!(height, 2606);
    }

    #[test]
    fn android_also_limits_total_pixels() {
        let (width, height) =
            export_view_size(2560, ANDROID_BUDGET, 2000, 20000).expect("需要导出副本");
        assert!(width < 2000);
        assert!((f64::from(height) / f64::from(width) - 10.0).abs() < 0.01);
    }

    #[test]
    fn desktop_limits_width_only() {
        // 桌面不做总像素预算：窄长图原样使用，宽图只按设置宽度缩。
        assert_eq!(export_view_size(1280, None, 1000, 40000), None);
        let (width, height) = export_view_size(1280, None, 2000, 20000).expect("需要导出副本");
        assert_eq!((width, height), (1280, 12800));
    }
}

