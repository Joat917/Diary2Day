use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::State;
use tauri_plugin_android_fs::{api::api_sync::AndroidFs, AndroidFsExt, FsUri};

use crate::commands::run_blocking;
use crate::error::AppError;
use crate::export::{self, BackupReport, ExportReport, ImportReport};
use crate::state::AppState;

type Fs = AndroidFs<tauri::Wry>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickedBackup {
    pub path: String,
}

fn android_err(context: &str, err: impl std::fmt::Display) -> AppError {
    AppError::new("android_io", context.to_string()).with_detail(err.to_string())
}

fn staging_dir(state: &AppState, name: &str) -> Result<PathBuf, AppError> {
    let dir = state.paths.cache_dir.join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn clear_staging(state: &AppState, name: &str) {
    let _ = fs::remove_dir_all(state.paths.cache_dir.join(name));
}

fn safe_name(raw: &str) -> String {
    let name = raw.rsplit(['/', '\\']).next().unwrap_or(raw).trim();
    if name.is_empty() || name == "." || name == ".." {
        "Diary2Day-backup.zip".to_string()
    } else {
        name.to_string()
    }
}

fn copy_local_dir(api: &Fs, source: &Path, target: &FsUri) -> Result<(), AppError> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            let (child, _) = api
                .create_dir_all_and_return_relative_path(target, &name)
                .map_err(|err| android_err("无法在所选目录创建子文件夹", err))?;
            copy_local_dir(api, &path, &child)?;
        } else {
            let uri = api
                .create_new_file(target, &name, None)
                .map_err(|err| android_err("无法在所选目录创建文件", err))?;
            let mut out = api
                .open_file_writable(&uri)
                .map_err(|err| android_err("无法写入所选目录", err))?;
            let mut input = fs::File::open(&path)?;
            std::io::copy(&mut input, &mut out)?;
            out.flush()?;
        }
    }
    Ok(())
}

fn copy_saf_dir(api: &Fs, source: &FsUri, target: &Path) -> Result<(), AppError> {
    let entries = api
        .read_dir(source)
        .map_err(|err| android_err("无法读取所选文件夹", err))?;
    for entry in entries {
        let path = target.join(safe_name(entry.name()));
        if entry.is_dir() {
            fs::create_dir_all(&path)?;
            copy_saf_dir(api, entry.uri(), &path)?;
        } else {
            let mut input = api
                .open_file_readable(entry.uri())
                .map_err(|err| android_err("无法读取所选文件", err))?;
            let mut out = fs::File::create(&path)?;
            std::io::copy(&mut input, &mut out)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn data_export_android(
    state: State<'_, AppState>,
) -> Result<Option<ExportReport>, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| {
        let api = state.app.android_fs();
        let Some(tree) = api
            .picker()
            .pick_dir(None, false)
            .map_err(|err| android_err("无法打开目录选择器", err))?
        else {
            return Ok(None);
        };

        let staging = staging_dir(&state, "export")?;
        let report = export::export_dir(&state, &staging.to_string_lossy());
        let report = match report {
            Ok(report) => report,
            Err(err) => {
                clear_staging(&state, "export");
                return Err(err);
            }
        };

        let out_dir = PathBuf::from(&report.dir);
        let folder = out_dir
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Diary2Day".to_string());
        let written = api
            .create_new_dir_and_return_relative_path(&tree, &folder)
            .map_err(|err| android_err("无法在所选目录创建文件夹", err))
            .and_then(|(dir, relative)| {
                copy_local_dir(api, &out_dir, &dir)?;
                let base = api.get_name_or_last_path_segment(&tree);
                Ok(format!("{base}/{}", relative.to_string_lossy()))
            });
        clear_staging(&state, "export");

        Ok(Some(ExportReport {
            dir: written?,
            journals: report.journals,
            assets: report.assets,
        }))
    })
    .await
}

#[tauri::command]
pub async fn data_import_android(
    state: State<'_, AppState>,
) -> Result<Option<ImportReport>, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| {
        let api = state.app.android_fs();
        let Some(tree) = api
            .picker()
            .pick_dir(None, false)
            .map_err(|err| android_err("无法打开目录选择器", err))?
        else {
            return Ok(None);
        };

        let staging = staging_dir(&state, "import")?;
        let result = copy_saf_dir(api, &tree, &staging)
            .and_then(|_| export::import_dir(&state, &staging.to_string_lossy()));
        clear_staging(&state, "import");

        Ok(Some(result?))
    })
    .await
}

#[tauri::command]
pub async fn data_backup_android(
    state: State<'_, AppState>,
) -> Result<Option<BackupReport>, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| {
        let api = state.app.android_fs();
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let name = format!("Diary2Day-Backup-{stamp}.zip");

        let Some(uri) = api
            .picker()
            .save_file(None, &name, Some("application/zip"), false)
            .map_err(|err| android_err("无法打开保存对话框", err))?
        else {
            return Ok(None);
        };

        let staging = staging_dir(&state, "backup-out")?;
        let target = staging.join(&name);
        let result = export::backup(&state, &target.to_string_lossy()).and_then(|report| {
            let mut input = fs::File::open(&target)?;
            let mut out = api
                .open_file_writable(&uri)
                .map_err(|err| android_err("无法写入所选文件", err))?;
            std::io::copy(&mut input, &mut out)?;
            out.flush()?;
            Ok(BackupReport {
                path: api.get_name_or_last_path_segment(&uri),
                size: report.size,
            })
        });
        clear_staging(&state, "backup-out");

        Ok(Some(result?))
    })
    .await
}

#[tauri::command]
pub async fn data_pick_restore_android(
    state: State<'_, AppState>,
) -> Result<Option<PickedBackup>, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| {
        let api = state.app.android_fs();
        // 部分文件管理器把 zip 标注为 x-zip-compressed；MIME 过滤本身不保证精确匹配。
        let Some(uri) = api
            .picker()
            .pick_file(
                None,
                &[
                    "application/zip",
                    "application/x-zip-compressed",
                    "application/x-zip",
                    "application/octet-stream",
                ],
                false,
            )
            .map_err(|err| android_err("无法打开文件选择器", err))?
        else {
            return Ok(None);
        };

        let name = safe_name(&api.get_name_or_last_path_segment(&uri));
        let staging = staging_dir(&state, "restore-pick")?;
        let target = staging.join(&name);
        let copied = (|| -> Result<(), AppError> {
            let mut input = api
                .open_file_readable(&uri)
                .map_err(|err| android_err("无法读取所选备份文件", err))?;
            let mut out = fs::File::create(&target)?;
            std::io::copy(&mut input, &mut out)?;
            Ok(())
        })();
        if let Err(err) = copied {
            clear_staging(&state, "restore-pick");
            return Err(err);
        }

        Ok(Some(PickedBackup {
            path: target.to_string_lossy().into_owned(),
        }))
    })
    .await
}

#[tauri::command]
pub async fn data_restore_android(
    state: State<'_, AppState>,
    path: String,
) -> Result<ImportReport, AppError> {
    let st = state.inner().clone();
    run_blocking(st, move |state| {
        let result = export::restore(&state, &path);
        // 失败时保留暂存 zip，便于升级后重试；下次选择备份时统一清空。
        if result.is_ok() {
            clear_staging(&state, "restore-pick");
        }
        result
    })
    .await
}