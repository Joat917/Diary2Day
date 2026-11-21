use base64::Engine;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFile {
    pub orig_name: String,
    pub data: String,
}

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

fn decode_payload(data: &str) -> Result<Vec<u8>, AppError> {
    let raw = data
        .split_once("base64,")
        .map(|(_, value)| value)
        .unwrap_or(data);
    base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(raw.trim()))
        .map_err(|err| AppError::new("bad_asset", "图片数据无法读取").with_detail(err.to_string()))
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

pub fn import_bytes(
    state: &AppState,
    orig_name: &str,
    bytes: &[u8],
) -> Result<AssetDto, AppError> {
    if bytes.is_empty() {
        return Err(AppError::new("bad_asset", "附件内容为空"));
    }
    if bytes.len() > 64 * 1024 * 1024 {
        return Err(AppError::new("asset_too_large", "单个附件不能超过 64MB"));
    }
    let ext = ext_of(orig_name)
        .filter(|ext| kind_for(ext).is_some())
        .or_else(|| sniff_ext(bytes).map(str::to_string))
        .ok_or_else(|| AppError::new("bad_asset", "无法识别文件类型"))?;
    let kind = kind_for(&ext)
        .ok_or_else(|| AppError::new("bad_asset", "不支持的文件类型"))?
        .to_string();

    let stem = uuid::Uuid::now_v7().simple().to_string();
    let name = format!("{stem}.{ext}");
    let path: PathBuf = state.paths.assets_dir.join(&name);

    let mut handle = fs::File::create(&path)?;
    handle.write_all(bytes)?;
    handle.flush()?;
    drop(handle);

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

    let hash = Sha256::digest(bytes);
    let size = bytes.len() as i64;
    state.with_db(|conn| {
        conn.execute(
            "INSERT INTO asset(name, kind, ext, size, width, height, duration_ms, hash, \
             orig_name, created_at) VALUES(?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?9)",
            params![
                name,
                kind,
                ext,
                size,
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
        size: bytes.len() as u64,
        width,
        height,
    })
}

pub fn import_files(state: &AppState, files: Vec<ImportFile>) -> Result<Vec<AssetDto>, AppError> {
    let mut out = Vec::new();
    for file in files {
        let bytes = decode_payload(&file.data)?;
        if bytes.is_empty() {
            continue;
        }
        out.push(import_bytes(state, &file.orig_name, &bytes)?);
    }
    Ok(out)
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

