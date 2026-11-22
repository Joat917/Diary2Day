#[cfg(not(target_os = "android"))]
use std::fs::File;

use tauri::State;

use crate::{
    commands::run_blocking,
    error::AppError,
    fs::appearance::{self, FontInfo, ThemeBackground},
    state::AppState,
};

#[cfg(not(target_os = "android"))]
const FONT_EXTENSIONS: &[&str] = &["ttf", "otf", "woff", "woff2"];
#[cfg(not(target_os = "android"))]
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp"];

#[tauri::command]
pub async fn font_import(state: State<'_, AppState>) -> Result<Option<FontInfo>, AppError> {
    let st = state.inner().clone();
    run_blocking(st, |state| pick_font(&state)).await
}

#[tauri::command]
pub async fn font_remove(state: State<'_, AppState>, name: String) -> Result<(), AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| appearance::remove_font(&state, &name)).await
}

#[tauri::command]
pub async fn theme_bg_import(
    state: State<'_, AppState>,
) -> Result<Option<ThemeBackground>, AppError> {
    let st = state.inner().clone();
    run_blocking(st, |state| pick_theme_bg(&state)).await
}

#[tauri::command]
pub async fn theme_bg_remove(state: State<'_, AppState>, name: String) -> Result<(), AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| appearance::remove_theme_bg(&state, &name)).await
}

#[cfg(not(target_os = "android"))]
fn pick_font(state: &AppState) -> Result<Option<FontInfo>, AppError> {
    use tauri_plugin_dialog::DialogExt;

    let builder = state
        .app
        .dialog()
        .file()
        .add_filter("Fonts", FONT_EXTENSIONS);
    #[cfg(any(windows, target_os = "macos"))]
    let builder = {
        use tauri::Manager;
        match state.app.get_webview_window("main") {
            Some(window) => builder.set_parent(&window),
            None => builder,
        }
    };
    let Some(file) = builder.blocking_pick_file() else {
        return Ok(None);
    };
    let path = file.into_path().map_err(|err| {
        AppError::new("bad_font", "无法读取所选文件路径").with_detail(err.to_string())
    })?;
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut input = File::open(&path)?;
    appearance::import_font_reader(state, &name, &mut input).map(Some)
}

#[cfg(target_os = "android")]
fn pick_font(state: &AppState) -> Result<Option<FontInfo>, AppError> {
    use tauri_plugin_android_fs::AndroidFsExt;

    let api = state.app.android_fs();
    let uris = api
        .picker()
        .pick_files(None, &["*/*"], false)
        .map_err(|err| {
            AppError::new("android_io", "无法打开文件选择器").with_detail(err.to_string())
        })?;
    let Some(uri) = uris.into_iter().next() else {
        return Ok(None);
    };
    let name = api.get_name_or_last_path_segment(&uri);
    let mut input = api.open_file_readable(&uri).map_err(|err| {
        AppError::new("android_io", "无法读取所选文件").with_detail(err.to_string())
    })?;
    appearance::import_font_reader(state, &name, &mut input).map(Some)
}

#[cfg(not(target_os = "android"))]
fn pick_theme_bg(state: &AppState) -> Result<Option<ThemeBackground>, AppError> {
    use tauri_plugin_dialog::DialogExt;

    let builder = state
        .app
        .dialog()
        .file()
        .add_filter("Images", IMAGE_EXTENSIONS);
    #[cfg(any(windows, target_os = "macos"))]
    let builder = {
        use tauri::Manager;
        match state.app.get_webview_window("main") {
            Some(window) => builder.set_parent(&window),
            None => builder,
        }
    };
    let Some(file) = builder.blocking_pick_file() else {
        return Ok(None);
    };
    let path = file.into_path().map_err(|err| {
        AppError::new("bad_image", "无法读取所选文件路径").with_detail(err.to_string())
    })?;
    let mut input = File::open(&path)?;
    appearance::import_theme_bg_reader(state, &mut input).map(Some)
}

#[cfg(target_os = "android")]
fn pick_theme_bg(state: &AppState) -> Result<Option<ThemeBackground>, AppError> {
    use tauri_plugin_android_fs::AndroidFsExt;

    let api = state.app.android_fs();
    let uris = api
        .picker()
        .pick_files(None, &["image/*"], false)
        .map_err(|err| {
            AppError::new("android_io", "无法打开图片选择器").with_detail(err.to_string())
        })?;
    let Some(uri) = uris.into_iter().next() else {
        return Ok(None);
    };
    let mut input = api.open_file_readable(&uri).map_err(|err| {
        AppError::new("android_io", "无法读取所选图片").with_detail(err.to_string())
    })?;
    appearance::import_theme_bg_reader(state, &mut input).map(Some)
}