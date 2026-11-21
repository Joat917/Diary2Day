use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use rusqlite::Connection;
use tauri::{AppHandle, Manager};

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub assets_dir: PathBuf,
    #[allow(dead_code)]
    pub cache_dir: PathBuf,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub paths: AppPaths,
    pub app: AppHandle,
}

impl AppState {
    pub fn init(app: &AppHandle) -> Result<Self, AppError> {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|err| AppError::io(err.to_string()))?;
        let assets_dir = data_dir.join("assets");
        let cache_dir = data_dir.join("cache");
        fs::create_dir_all(&assets_dir)?;
        fs::create_dir_all(&cache_dir)?;
        let db_path = data_dir.join("diary.db");

        let mut conn = crate::db::conn::open(&db_path)?;
        crate::db::migrate::run(&mut conn)?;

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            paths: AppPaths {
                data_dir,
                db_path,
                assets_dir,
                cache_dir,
            },
            app: app.clone(),
        })
    }

    pub fn with_db<T>(
        &self,
        f: impl FnOnce(&mut Connection) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let mut conn = self
            .db
            .lock()
            .map_err(|_| AppError::internal("数据库锁已损坏"))?;
        f(&mut conn)
    }
}

pub fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| {
            let path = entry.path();
            if path.is_dir() {
                dir_size(&path)
            } else {
                entry.metadata().map(|meta| meta.len()).unwrap_or(0)
            }
        })
        .sum()
}
