pub mod app;
pub mod journal;
pub mod search;
pub mod settings;
pub mod tag;

use crate::{error::AppError, state::AppState};

pub async fn run_blocking<T, F>(state: AppState, f: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce(AppState) -> Result<T, AppError> + Send + 'static,
{
    tokio::task::spawn_blocking(move || f(state))
        .await
        .map_err(|err| AppError::internal("后台任务异常").with_detail(err.to_string()))?
}
