use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::db::journal::{self, SaveInput};
use crate::error::AppError;
use crate::fs::assets::{self, AssetDto};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
struct ShareFile {
    path: String,
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct SharePayload {
    #[serde(default)]
    text: String,
    #[serde(default)]
    files: Vec<ShareFile>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareReport {
    pub id: String,
    pub day_key: String,
    pub assets: usize,
}

fn inbox_dir(state: &AppState) -> PathBuf {
    state.paths.cache_dir.join("share-inbox")
}

fn display_name(file: &ShareFile, path: &Path) -> String {
    let original = file
        .name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(120)
        .collect::<String>();
    if original.is_empty() {
        return path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "shared".to_string());
    }
    original
}

fn take_file(state: &AppState, root: &Path, file: &ShareFile) -> Option<AssetDto> {
    let canonical = PathBuf::from(&file.path).canonicalize().ok()?;
    if !canonical.starts_with(root) || !canonical.is_file() {
        return None;
    }
    let mut input = fs::File::open(&canonical).ok()?;
    assets::import_reader(state, &display_name(file, &canonical), &mut input).ok()
}

fn asset_markdown(asset: &AssetDto) -> String {
    let url = format!("ddasset://{}/{}", asset.kind, asset.name);
    if asset.kind == "image" {
        format!("![]({url})")
    } else {
        format!("<{} controls src=\"{url}\"></{}>", asset.kind, asset.kind)
    }
}

fn discard_bad_pending(pending: &Path) {
    let bad = pending.with_file_name("pending.bad.json");
    let _ = fs::remove_file(&bad);
    let _ = fs::rename(pending, bad);
}

fn cleanup(pending: &Path, root: &Path, payloads: &[SharePayload]) {
    for payload in payloads {
        for file in &payload.files {
            let path = PathBuf::from(&file.path);
            let canonical = match path.canonicalize() {
                Ok(path) => path,
                Err(_) => continue,
            };
            if canonical.starts_with(root) {
                let _ = fs::remove_file(canonical);
            }
        }
    }
    let _ = fs::remove_file(pending);
}

pub fn drain(state: &AppState) -> Result<Option<ShareReport>, AppError> {
    let inbox = inbox_dir(state);
    let pending = inbox.join("pending.json");
    let raw = match fs::read_to_string(&pending) {
        Ok(raw) => raw,
        Err(_) => return Ok(None),
    };
    let payloads: Vec<SharePayload> = match serde_json::from_str(&raw) {
        Ok(payloads) => payloads,
        Err(_) => {
            discard_bad_pending(&pending);
            return Ok(None);
        }
    };
    let root = inbox.canonicalize().unwrap_or_else(|_| inbox.clone());

    let mut texts = Vec::new();
    let mut assets = Vec::new();
    for payload in &payloads {
        let text = payload.text.trim();
        if !text.is_empty() {
            texts.push(text.to_string());
        }
        for file in &payload.files {
            if let Some(asset) = take_file(state, &root, file) {
                assets.push(asset);
            }
        }
    }

    let mut parts = Vec::new();
    if !texts.is_empty() {
        parts.push(texts.join("\n\n"));
    }
    parts.extend(assets.iter().map(asset_markdown));
    let content = parts.join("\n\n");
    if content.trim().is_empty() {
        cleanup(&pending, &root, &payloads);
        return Ok(None);
    }

    let journal = state.with_db(|conn| {
        journal::save(
            conn,
            SaveInput {
                id: None,
                content,
                format: Some("markdown".to_string()),
                title: None,
                mood: None,
                weather: None,
                tag_ids: Vec::new(),
                recorded_at: None,
                recorded_off: None,
                pinned: false,
                sensitive: false,
                revision: None,
            },
        )
    })?;

    cleanup(&pending, &root, &payloads);
    Ok(Some(ShareReport {
        id: journal.id,
        day_key: journal.day_key,
        assets: assets.len(),
    }))
}