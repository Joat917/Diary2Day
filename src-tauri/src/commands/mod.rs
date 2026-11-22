#[cfg(target_os = "android")]
pub mod android_data;
pub mod app;
pub mod appearance;
pub mod data;
pub mod image;
pub mod journal;
pub mod lock;
pub mod media;
pub mod search;
pub mod settings;
pub mod share;
pub mod tag;

use crate::{error::AppError, state::AppState};

pub async fn run_blocking_public<T, F>(state: AppState, f: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce(AppState) -> Result<T, AppError> + Send + 'static,
{
    tokio::task::spawn_blocking(move || f(state))
        .await
        .map_err(|err| AppError::internal("后台任务异常").with_detail(err.to_string()))?
}

pub async fn run_blocking<T, F>(state: AppState, f: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce(AppState) -> Result<T, AppError> + Send + 'static,
{
    if state.is_locked() {
        return Err(AppError::new("locked", "应用已锁定"));
    }
    run_blocking_public(state, f).await
}