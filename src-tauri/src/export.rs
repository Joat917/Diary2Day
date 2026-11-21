use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;
use tauri::Emitter;

use crate::db::{export as db_export, journal, tag};
use crate::error::AppError;
use crate::events::{self, OpProgress};
use crate::fs::assets::{kind_for, scan_asset_refs};
use crate::state::{AppState, LockRuntime};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    pub dir: String,
    pub journals: i64,
    pub assets: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub journals: i64,
    pub assets: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupReport {
    pub path: String,
    pub size: u64,
}

fn emit_progress(state: &AppState, op_id: &str, phase: &str, done: i64, total: i64) {
    let _ = state.app.emit(
        events::OP_PROGRESS,
        OpProgress {
            op_id: op_id.to_string(),
            phase: phase.to_string(),
            done,
            total,
        },
    );
}

fn quote(value: &str) -> String {
    value.replace(['\r', '\n'], " ").trim().to_string()
}

pub fn export_dir(state: &AppState, target_dir: &str) -> Result<ExportReport, AppError> {
    let base = PathBuf::from(target_dir);
    if target_dir.trim().is_empty() || !base.is_dir() {
        return Err(AppError::new("bad_target", "请选择有效的导出目录"));
    }
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let out_dir = base.join(format!("Diary2Day-{stamp}"));
    fs::create_dir_all(out_dir.join("assets"))?;
    let op_id = uuid::Uuid::now_v7().to_string();

    let entries = state.with_db(|conn| db_export::live_entries(conn))?;
    let total = entries.len() as i64;
    let mut copied: Vec<String> = Vec::new();

    for (index, entry) in entries.iter().enumerate() {
        let mut body = entry.content.clone();
        for (kind, name) in scan_asset_refs(&entry.content) {
            body = body.replace(
                &format!("ddasset://{kind}/{name}"),
                &format!("assets/{name}"),
            );
            if !copied.contains(&name) {
                let source = state.paths.assets_dir.join(&name);
                if source.is_file() {
                    fs::copy(&source, out_dir.join("assets").join(&name))?;
                    copied.push(name);
                }
            }
        }
        let yaml = format!(
            "---\nid: {}\nrecorded_at: {}\noffset: {}\nformat: {}\nmood: {}\npinned: {}\n\
             sensitive: {}\ntags: {}\nwords: {}\n---\n\n",
            entry.id,
            entry.recorded_at,
            entry.recorded_off,
            entry.format,
            entry.mood.clone().unwrap_or_default(),
            entry.pinned,
            entry.sensitive,
            quote(&entry.tags.join(", ")),
            entry.words
        );
        let filename = format!("{}-{}.md", entry.day_key, entry.id);
        let mut file = fs::File::create(out_dir.join(filename))?;
        file.write_all(yaml.as_bytes())?;
        file.write_all(body.as_bytes())?;
        file.flush()?;

        if index % 20 == 0 || index + 1 == entries.len() {
            emit_progress(state, &op_id, "export", index as i64 + 1, total);
        }
    }

    emit_progress(state, &op_id, "export", total, total);
    Ok(ExportReport {
        dir: out_dir.to_string_lossy().into_owned(),
        journals: total,
        assets: copied.len() as i64,
    })
}

struct ParsedMd {
    id: Option<String>,
    recorded_at: Option<String>,
    offset: Option<i32>,
    mood: Option<String>,
    pinned: bool,
    sensitive: bool,
    tags: Vec<String>,
    content: String,
}

fn parse_md(raw: &str) -> ParsedMd {
    let mut parsed = ParsedMd {
        id: None,
        recorded_at: None,
        offset: None,
        mood: None,
        pinned: false,
        sensitive: false,
        tags: Vec::new(),
        content: raw.to_string(),
    };
    let Some(rest) = raw.strip_prefix("---") else {
        return parsed;
    };
    let rest = rest.strip_prefix("\r\n").or_else(|| rest.strip_prefix('\n'));
    let Some(rest) = rest else {
        return parsed;
    };
    let Some(end) = rest.find("\n---") else {
        return parsed;
    };
    let header = &rest[..end];
    let body_start = end + 4;
    parsed.content = rest[body_start..]
        .strip_prefix("\r\n")
        .or_else(|| rest[body_start..].strip_prefix('\n'))
        .unwrap_or(&rest[body_start..])
        .to_string();

    for line in header.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"').trim_matches('\'').to_string();
        match key {
            "id" if !value.is_empty() => parsed.id = Some(value),
            "recorded_at" if !value.is_empty() => parsed.recorded_at = Some(value),
            "offset" => parsed.offset = value.parse().ok(),
            "mood" if !value.is_empty() => parsed.mood = Some(value),
            "pinned" => parsed.pinned = value == "true",
            "sensitive" => parsed.sensitive = value == "true",
            "tags" if !value.is_empty() => {
                let trimmed = value.trim_matches(['[', ']']);
                parsed.tags = trimmed
                    .split(',')
                    .map(|tag| tag.trim().trim_matches('"').trim().to_string())
                    .filter(|tag| !tag.is_empty())
                    .collect();
            }
            _ => {}
        }
    }
    parsed
}

fn restore_asset_links(content: &str, source_assets: &Path) -> (String, Vec<(String, String)>) {
    let mut out = content.to_string();
    let mut found: Vec<(String, String)> = Vec::new();
    let mut rest = content;
    while let Some(pos) = rest.find("assets/") {
        let after = &rest[pos + "assets/".len()..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'))
            .unwrap_or(after.len());
        let name = &after[..end];
        if let Some((stem, ext)) = name.rsplit_once('.')
            && !stem.is_empty()
            && !ext.is_empty()
            && source_assets.join(name).is_file()
            && let Some(kind) = kind_for(&ext.to_ascii_lowercase())
        {
            out = out.replace(&format!("assets/{name}"), &format!("ddasset://{kind}/{name}"));
            if !found.iter().any(|(_, existing)| existing == name) {
                found.push((kind.to_string(), name.to_string()));
            }
        }
        rest = &after[end..];
    }
    (out, found)
}

pub fn import_dir(state: &AppState, dir: &str) -> Result<ImportReport, AppError> {
    let base = PathBuf::from(dir);
    if !base.is_dir() {
        return Err(AppError::new("bad_target", "请选择有效的导入目录"));
    }
    let source_assets = base.join("assets");
    let op_id = uuid::Uuid::now_v7().to_string();
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(&base)?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("md") && path.is_file() {
            files.push(path);
        }
    }
    files.sort();

    let total = files.len() as i64;
    let mut journals = 0i64;
    let mut asset_total = 0i64;

    for (index, path) in files.iter().enumerate() {
        let raw = fs::read_to_string(path)?;
        let parsed = parse_md(&raw);
        let (content, refs) = restore_asset_links(&parsed.content, &source_assets);
        if content.trim().is_empty() {
            continue;
        }
        for (kind, name) in refs {
            let source = source_assets.join(&name);
            let target = state.paths.assets_dir.join(&name);
            if source.is_file() && !target.is_file() {
                fs::copy(&source, &target)?;
            }
            let ext = name
                .rsplit_once('.')
                .map(|(_, value)| value.to_ascii_lowercase())
                .unwrap_or_default();
            let size = fs::metadata(&target).map(|meta| meta.len() as i64).unwrap_or(0);
            state.with_db(|conn| {
                db_export::insert_asset_if_missing(conn, &name, &kind, &ext, size, &name)
            })?;
            asset_total += 1;
        }

        let tag_ids = state.with_db(|conn| {
            let mut ids = Vec::new();
            for name in &parsed.tags {
                if let Ok(tag) = tag::ensure(conn, name) {
                    ids.push(tag.id);
                }
            }
            Ok(ids)
        })?;

        let existing = parsed
            .id
            .as_ref()
            .and_then(|id| state.with_db(|conn| journal::get(conn, id)).ok())
            .map(|item| (item.id, item.revision));

        state.with_db(|conn| {
            journal::save(
                conn,
                journal::SaveInput {
                    id: existing.as_ref().map(|(id, _)| id.clone()),
                    content: content.clone(),
                    format: Some("markdown".into()),
                    title: None,
                    mood: parsed.mood.clone(),
                    tag_ids: tag_ids.clone(),
                    recorded_at: parsed.recorded_at.clone(),
                    recorded_off: parsed.offset,
                    pinned: parsed.pinned,
                    sensitive: parsed.sensitive,
                    revision: existing.as_ref().map(|(_, revision)| *revision),
                },
            )
        })?;
        journals += 1;
        if index % 20 == 0 || index + 1 == files.len() {
            emit_progress(state, &op_id, "import", index as i64 + 1, total);
        }
    }

    let _ = state.app.emit(
        events::DATA_CHANGED,
        events::DataChanged {
            kind: "all".into(),
            id: None,
            day_key: None,
        },
    );
    Ok(ImportReport {
        journals,
        assets: asset_total,
    })
}

fn walk_files(dir: &Path, base: &Path, out: &mut Vec<(PathBuf, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|value| value.to_str()) == Some(".thumbs") {
                continue;
            }
            walk_files(&path, base, out);
        } else if let Ok(relative) = path.strip_prefix(base) {
            out.push((path.clone(), relative.to_string_lossy().replace('\\', "/")));
        }
    }
}

pub fn backup(state: &AppState, target_path: &str) -> Result<BackupReport, AppError> {
    let target = PathBuf::from(target_path);
    if target.as_os_str().is_empty() {
        return Err(AppError::new("bad_target", "请选择备份文件位置"));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let backup_dir = state.paths.cache_dir.join("backup");
    fs::create_dir_all(&backup_dir)?;
    let snapshot = backup_dir.join("live.db");
    if snapshot.exists() {
        let _ = fs::remove_file(&snapshot);
    }
    let counts = state.with_db(|conn| {
        let journals: i64 = conn.query_row(
            "SELECT COUNT(*) FROM journal WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )?;
        let assets: i64 = conn.query_row("SELECT COUNT(*) FROM asset", [], |row| row.get(0))?;
        Ok((journals, assets, db_export::schema_version(conn)))
    })?;
    let snapshot_sql = snapshot.to_string_lossy().replace('\'', "''");
    state.with_db(|conn| {
        conn.execute_batch(&format!("VACUUM INTO '{snapshot_sql}'"))?;
        Ok(())
    })?;

    let manifest = serde_json::json!({
        "app_version": env!("CARGO_PKG_VERSION"),
        "schema_version": counts.2,
        "counts": { "journals": counts.0, "assets": counts.1 },
        "created_at": crate::domain::utc_now_iso(),
        "media_included": true,
    });

    let file = fs::File::create(&target)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("manifest.json", options)
        .map_err(|err| AppError::io(err.to_string()))?;
    zip.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;

    zip.start_file("diary.db", options)
        .map_err(|err| AppError::io(err.to_string()))?;
    let mut db_file = fs::File::open(&snapshot)?;
    std::io::copy(&mut db_file, &mut zip)?;

    let mut assets: Vec<(PathBuf, String)> = Vec::new();
    walk_files(&state.paths.assets_dir, &state.paths.assets_dir, &mut assets);
    for (path, relative) in assets {
        zip.start_file(format!("assets/{relative}"), options)
            .map_err(|err| AppError::io(err.to_string()))?;
        let mut source = fs::File::open(&path)?;
        std::io::copy(&mut source, &mut zip)?;
    }
    zip.finish()
        .map_err(|err| AppError::io(err.to_string()))?;

    let size = fs::metadata(&target).map(|meta| meta.len()).unwrap_or(0);
    Ok(BackupReport {
        path: target.to_string_lossy().into_owned(),
        size,
    })
}

pub fn restore(state: &AppState, zip_path: &str) -> Result<ImportReport, AppError> {
    let source = PathBuf::from(zip_path);
    if !source.is_file() {
        return Err(AppError::new("bad_target", "备份文件不存在"));
    }
    let file = fs::File::open(&source)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|err| AppError::io(err.to_string()))?;

    let manifest_text = {
        let mut entry = archive
            .by_name("manifest.json")
            .map_err(|_| AppError::new("bad_backup", "备份缺少 manifest.json"))?;
        let mut text = String::new();
        entry.read_to_string(&mut text)?;
        text
    };
    let manifest: serde_json::Value = serde_json::from_str(&manifest_text)?;
    let backup_schema = manifest
        .get("schema_version")
        .and_then(|value| value.as_i64())
        .unwrap_or(0);
    let current_schema = state.with_db(|conn| Ok(db_export::schema_version(conn)))?;
    if backup_schema > current_schema {
        return Err(AppError::new(
            "restore_version_mismatch",
            "备份来自更新版本，请先升级 Diary2Day",
        ));
    }

    let restore_dir = state.paths.cache_dir.join("restore");
    if restore_dir.exists() {
        fs::remove_dir_all(&restore_dir)?;
    }
    fs::create_dir_all(restore_dir.join("assets"))?;
    let mut restored_db: Option<PathBuf> = None;
    let mut asset_count = 0i64;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|err| AppError::io(err.to_string()))?;
        let Some(name) = entry.enclosed_name().map(|path| path.to_path_buf()) else {
            continue;
        };
        let name_text = name.to_string_lossy().replace('\\', "/");
        if name_text == "diary.db" {
            let target = restore_dir.join("diary.db");
            let mut out = fs::File::create(&target)?;
            std::io::copy(&mut entry, &mut out)?;
            restored_db = Some(target);
        } else if let Some(relative) = name_text.strip_prefix("assets/")
            && !relative.is_empty()
        {
            let target = state.paths.assets_dir.join(relative);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out = fs::File::create(&target)?;
            std::io::copy(&mut entry, &mut out)?;
            asset_count += 1;
        }
    }

    let Some(restored_db) = restored_db else {
        return Err(AppError::new("bad_backup", "备份中缺少 diary.db"));
    };

    let mut guard = state
        .db
        .lock()
        .map_err(|_| AppError::internal("数据库锁已损坏"))?;
    guard.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
    let placeholder = Connection::open_in_memory()?;
    let old = std::mem::replace(&mut *guard, placeholder);
    drop(old);
    let wal = PathBuf::from(format!("{}-wal", state.paths.db_path.to_string_lossy()));
    let shm = PathBuf::from(format!("{}-shm", state.paths.db_path.to_string_lossy()));
    let _ = fs::remove_file(&wal);
    let _ = fs::remove_file(&shm);
    fs::copy(&restored_db, &state.paths.db_path)?;
    let mut fresh = crate::db::conn::open(&state.paths.db_path)?;
    crate::db::migrate::run(&mut fresh)?;
    *guard = fresh;
    drop(guard);

    let lock_runtime = match state.with_db(|conn| crate::db::lock::load(conn))? {
        Some(row) => LockRuntime {
            enabled: true,
            salt: row.salt,
            hash: row.hash,
            locked: false,
        },
        None => LockRuntime::disabled(),
    };
    state.update_lock(lock_runtime);
    let _ = state.app.emit(events::LOCK_STATE, state.lock_state());

    let journals = state.with_db(|conn| {
        conn.query_row(
            "SELECT COUNT(*) FROM journal WHERE deleted_at IS NULL",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(AppError::from)
    })?;

    let _ = state.app.emit(
        events::DATA_CHANGED,
        events::DataChanged {
            kind: "all".into(),
            id: None,
            day_key: None,
        },
    );
    Ok(ImportReport {
        journals,
        assets: asset_count,
    })
}