use std::fs::{self, File};
use std::io::{BufWriter, Cursor, Read, Write};
use std::path::Path;

use image::imageops::FilterType;
use image::metadata::Orientation;
use image::{DynamicImage, ExtendedColorType, ImageDecoder, ImageEncoder, ImageReader};
use serde::Serialize;

use crate::db::settings::{is_safe_font_name, is_safe_image_name};
use crate::error::AppError;
use crate::state::AppState;

const FONT_DIR: &str = "fonts";
const THEME_DIR: &str = "theme";
const MAX_FONT_BYTES: u64 = 10 * 1024 * 1024;
// 原图只作转码输入，64MB 足够覆盖手机直出的大图，同时挡住误选的超大文件。
const MAX_BG_BYTES: u64 = 64 * 1024 * 1024;
const BG_MAX_EDGE: u32 = 2560;
const BG_QUALITY: u8 = 82;
// image 0.25 的 WebP 编码器只有无损模式，背景图改用同质量的 JPEG。
const BG_EXT: &str = "jpg";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontInfo {
    pub name: String,
    pub label: String,
    pub format: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeBackground {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

fn read_limited(
    reader: &mut impl Read,
    limit: u64,
    code: &str,
    msg: &str,
) -> Result<Vec<u8>, AppError> {
    let mut bytes = Vec::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        if bytes.len() as u64 + read as u64 > limit {
            return Err(AppError::new(code, msg).with_detail(format!("上限 {limit} 字节")));
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn font_format(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"wOFF") {
        Some("woff")
    } else if bytes.starts_with(b"wOF2") {
        Some("woff2")
    } else if bytes.starts_with(b"OTTO") {
        Some("otf")
    } else if bytes.starts_with(&[0x00, 0x01, 0x00, 0x00])
        || bytes.starts_with(b"true")
        || bytes.starts_with(b"typ1")
    {
        Some("ttf")
    } else {
        None
    }
}

fn font_label(orig_name: &str) -> String {
    let base = orig_name.rsplit(['/', '\\']).next().unwrap_or(orig_name);
    let stem = base
        .rsplit_once('.')
        .map(|(value, _)| value)
        .unwrap_or(base)
        .trim();
    let label: String = stem.chars().take(40).collect();
    if label.is_empty() {
        "自定义字体".into()
    } else {
        label
    }
}

pub fn import_font_reader(
    state: &AppState,
    orig_name: &str,
    reader: &mut impl Read,
) -> Result<FontInfo, AppError> {
    let bytes = read_limited(
        reader,
        MAX_FONT_BYTES,
        "font_too_large",
        "字体文件超过 10MB",
    )?;
    if bytes.is_empty() {
        return Err(AppError::new("bad_font", "字体文件为空"));
    }
    if bytes.starts_with(b"ttcf") {
        return Err(AppError::new("bad_font", "暂不支持字体集合（.ttc）"));
    }
    let format = font_format(&bytes).ok_or_else(|| {
        AppError::new("bad_font", "不支持的字体格式")
            .with_detail(format!("{orig_name}：仅支持 ttf / otf / woff / woff2"))
    })?;

    let dir = state.paths.assets_dir.join(FONT_DIR);
    fs::create_dir_all(&dir)?;
    let name = format!("{}.{}", uuid::Uuid::now_v7().simple(), format);
    let path = dir.join(&name);
    let mut file = File::create(&path)?;
    if let Err(err) = file.write_all(&bytes).and_then(|()| file.flush()) {
        drop(file);
        let _ = fs::remove_file(&path);
        return Err(err.into());
    }
    drop(file);

    Ok(FontInfo {
        name,
        label: font_label(orig_name),
        format: format.to_string(),
    })
}

fn decode_image(bytes: &[u8]) -> Result<DynamicImage, AppError> {
    let mut decoder = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|err| AppError::new("bad_image", "无法读取图片").with_detail(err.to_string()))?
        .into_decoder()
        .map_err(|err| AppError::new("bad_image", "无法解码图片").with_detail(err.to_string()))?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decoder).map_err(|err| {
        AppError::new("bad_image", "无法解码图片").with_detail(err.to_string())
    })?;
    image.apply_orientation(orientation);
    Ok(image)
}

pub fn import_theme_bg_reader(
    state: &AppState,
    reader: &mut impl Read,
) -> Result<ThemeBackground, AppError> {
    let bytes = read_limited(reader, MAX_BG_BYTES, "bad_image", "背景图片超过 64MB")?;
    if bytes.is_empty() {
        return Err(AppError::new("bad_image", "背景图片为空"));
    }
    let mut image = decode_image(&bytes)?;
    if image.width().max(image.height()) > BG_MAX_EDGE {
        image = image.resize(BG_MAX_EDGE, BG_MAX_EDGE, FilterType::Lanczos3);
    }

    let dir = state.paths.assets_dir.join(THEME_DIR);
    fs::create_dir_all(&dir)?;
    let name = format!("{}.{BG_EXT}", uuid::Uuid::now_v7().simple());
    let path = dir.join(&name);
    let rgb = image.to_rgb8();
    let (width, height) = rgb.dimensions();
    let mut writer = BufWriter::new(File::create(&path)?);
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, BG_QUALITY);
    if let Err(err) = encoder.write_image(rgb.as_raw(), width, height, ExtendedColorType::Rgb8) {
        drop(writer);
        let _ = fs::remove_file(&path);
        return Err(
            AppError::new("bad_image", "背景图片无法保存").with_detail(err.to_string())
        );
    }
    if let Err(err) = writer.flush() {
        drop(writer);
        let _ = fs::remove_file(&path);
        return Err(err.into());
    }

    Ok(ThemeBackground {
        name,
        width,
        height,
    })
}

fn remove_if_exists(path: &Path) -> Result<(), AppError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(AppError::io(err.to_string())),
    }
}

pub fn remove_font(state: &AppState, name: &str) -> Result<(), AppError> {
    if !is_safe_font_name(name) {
        return Err(
            AppError::new("bad_font", "字体文件名不合法").with_detail(name.to_string())
        );
    }
    remove_if_exists(&state.paths.assets_dir.join(FONT_DIR).join(name))
}

pub fn remove_theme_bg(state: &AppState, name: &str) -> Result<(), AppError> {
    if !is_safe_image_name(name) {
        return Err(
            AppError::new("bad_image", "背景图文件名不合法").with_detail(name.to_string())
        );
    }
    remove_if_exists(&state.paths.assets_dir.join(THEME_DIR).join(name))
}

#[cfg(test)]
mod tests {
    use super::{font_format, font_label};

    #[test]
    fn sniffs_font_formats() {
        assert_eq!(font_format(&[0x00, 0x01, 0x00, 0x00, 0x00]), Some("ttf"));
        assert_eq!(font_format(b"true\x00\x00\x00"), Some("ttf"));
        assert_eq!(font_format(b"OTTO\x00\x00\x00"), Some("otf"));
        assert_eq!(font_format(b"wOFF\x00\x00\x00"), Some("woff"));
        assert_eq!(font_format(b"wOF2\x00\x00\x00"), Some("woff2"));
        assert_eq!(font_format(b"ttcf\x00\x00\x00"), None);
        assert_eq!(font_format(b"\x89PNG\r\n\x1a\n"), None);
    }

    #[test]
    fn labels_drop_path_and_extension() {
        assert_eq!(font_label("C:\\fonts\\LXGWWenKai-Regular.ttf"), "LXGWWenKai-Regular");
        assert_eq!(font_label("/tmp/霞鹜文楷.woff2"), "霞鹜文楷");
        assert_eq!(font_label("font.ttf"), "font");
    }

    #[test]
    fn labels_are_capped_at_40_chars() {
        let label = font_label(&format!("{}.ttf", "字".repeat(50)));
        assert_eq!(label.chars().count(), 40);
    }
}