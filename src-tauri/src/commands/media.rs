use std::path::Path;

use tauri::State;

use crate::{
    commands::run_blocking,
    error::AppError,
    fs::assets::{self, AssetDto, CleanupReport},
    state::AppState,
};

#[tauri::command]
pub async fn media_pick_import(state: State<'_, AppState>) -> Result<Vec<AssetDto>, AppError> {
    let st = state.inner().clone();
    run_blocking(st, |state| pick_impl(&state)).await
}

#[tauri::command]
pub async fn media_paste_import(state: State<'_, AppState>) -> Result<Vec<AssetDto>, AppError> {
    let st = state.inner().clone();
    run_blocking(st, |state| paste_impl(&state)).await
}

#[tauri::command]
pub async fn media_import_paths(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<Vec<AssetDto>, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| {
        let mut out = Vec::new();
        for path in paths {
            if let Some(asset) = assets::import_path(&state, Path::new(&path))? {
                out.push(asset);
            }
        }
        Ok(out)
    })
    .await
}

#[tauri::command]
pub async fn asset_gc(state: State<'_, AppState>, names: Vec<String>) -> Result<usize, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| assets::gc_unused(&state, names)).await
}

#[tauri::command]
pub async fn asset_clean_unused(state: State<'_, AppState>) -> Result<CleanupReport, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| assets::clean_unused(&state)).await
}

#[tauri::command]
pub async fn asset_export_views(
    state: State<'_, AppState>,
    names: Vec<String>,
) -> Result<Vec<assets::ExportViewDto>, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| assets::export_views(&state, names)).await
}

#[cfg(not(target_os = "android"))]
fn pick_impl(state: &AppState) -> Result<Vec<AssetDto>, AppError> {
    use tauri_plugin_dialog::DialogExt;

    let builder = state
        .app
        .dialog()
        .file()
        .add_filter("Media", assets::MEDIA_EXTENSIONS);
    #[cfg(any(windows, target_os = "macos"))]
    let builder = {
        use tauri::Manager;
        match state.app.get_webview_window("main") {
            Some(window) => builder.set_parent(&window),
            None => builder,
        }
    };
    let picked = builder.blocking_pick_files();
    let Some(files) = picked else {
        return Ok(Vec::new());
    };

    let mut out = Vec::new();
    for file in files {
        let path = file.into_path().map_err(|err| {
            AppError::new("bad_asset", "无法读取所选文件路径").with_detail(err.to_string())
        })?;
        if let Some(asset) = assets::import_path(state, &path)? {
            out.push(asset);
        }
    }
    Ok(out)
}

#[cfg(target_os = "android")]
fn pick_impl(state: &AppState) -> Result<Vec<AssetDto>, AppError> {
    use tauri_plugin_android_fs::AndroidFsExt;

    let api = state.app.android_fs();
    let uris = api
        .picker()
        .pick_files(None, &["image/*", "audio/*", "video/*"], false)
        .map_err(|err| {
            AppError::new("android_io", "无法打开文件选择器").with_detail(err.to_string())
        })?;

    let mut out = Vec::new();
    for uri in uris {
        let name = api.get_name_or_last_path_segment(&uri);
        if !assets::is_media_name(&name) {
            continue;
        }
        let mut input = api.open_file_readable(&uri).map_err(|err| {
            AppError::new("android_io", "无法读取所选文件").with_detail(err.to_string())
        })?;
        out.push(assets::import_reader(state, &name, &mut input)?);
    }
    Ok(out)
}

#[cfg(not(target_os = "android"))]
fn paste_impl(state: &AppState) -> Result<Vec<AssetDto>, AppError> {
    let mut clipboard = arboard::Clipboard::new().map_err(|err| {
        AppError::new("clipboard", "无法读取剪贴板").with_detail(err.to_string())
    })?;

    if let Ok(paths) = clipboard.get().file_list() {
        if !paths.is_empty() {
            let mut out = Vec::new();
            for path in paths {
                if let Some(asset) = assets::import_path(state, &path)? {
                    out.push(asset);
                }
            }
            if !out.is_empty() {
                return Ok(out);
            }
        }
    }

    let Ok(image) = clipboard.get().image() else {
        return Ok(Vec::new());
    };
    let buffer = image::RgbaImage::from_raw(
        image.width as u32,
        image.height as u32,
        image.bytes.into_owned(),
    )
    .ok_or_else(|| AppError::new("bad_asset", "剪贴板图片尺寸无效"))?;
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(buffer)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|err| {
            AppError::new("bad_asset", "剪贴板图片无法编码").with_detail(err.to_string())
        })?;
    Ok(vec![assets::import_bytes(state, "pasted.png", &png)?])
}

#[cfg(target_os = "android")]
fn paste_impl(_state: &AppState) -> Result<Vec<AssetDto>, AppError> {
    Err(AppError::new(
        "paste_media_unsupported",
        "该平台不支持粘贴媒体，请用附件按钮选择文件",
    ))
}