use tauri::{Emitter, State};

use crate::{
    commands::run_blocking_public,
    db::lock::{self, LockState},
    error::AppError,
    events::LOCK_STATE,
    state::{AppState, LockRuntime},
};

fn current_row(state: &AppState) -> Result<lock::LockRow, AppError> {
    let guard = state
        .lock
        .lock()
        .map_err(|_| AppError::internal("锁状态不可用"))?;
    if !guard.enabled {
        return Err(AppError::new("lock_disabled", "应用锁尚未启用"));
    }
    Ok(lock::LockRow {
        salt: guard.salt.clone(),
        hash: guard.hash.clone(),
    })
}

fn emit(state: &AppState, next: LockState) {
    let _ = state.app.emit(LOCK_STATE, next);
}

#[tauri::command]
pub async fn app_lock_status(state: State<'_, AppState>) -> Result<LockState, AppError> {
    Ok(state.lock_state())
}

#[tauri::command]
pub async fn app_lock_set(
    state: State<'_, AppState>,
    code: String,
) -> Result<LockState, AppError> {
    lock::validate_code(&code)?;
    let salt = uuid::Uuid::now_v7().simple().to_string();
    let hash = lock::derive(&salt, &code);
    let st = state.inner().clone();
    let salt_for_db = salt.clone();
    let hash_for_db = hash.clone();
    run_blocking_public(st.clone(), move |state| {
        state.with_db(|conn| lock::save(conn, &salt_for_db, &hash_for_db))
    })
    .await?;

    st.update_lock(LockRuntime {
        enabled: true,
        salt,
        hash,
        locked: false,
    });
    let next = st.lock_state();
    emit(&st, next.clone());
    Ok(next)
}

#[tauri::command]
pub async fn app_lock_clear(
    state: State<'_, AppState>,
    code: String,
) -> Result<LockState, AppError> {
    let row = current_row(state.inner())?;
    if !lock::verify(&row, &code) {
        return Err(AppError::new("bad_lock_code", "口令不正确"));
    }
    let st = state.inner().clone();
    run_blocking_public(st.clone(), move |state| {
        state.with_db(|conn| lock::clear(conn))
    })
    .await?;
    st.update_lock(LockRuntime::disabled());
    let next = st.lock_state();
    emit(&st, next.clone());
    Ok(next)
}

#[tauri::command]
pub async fn app_lock_unlock(
    state: State<'_, AppState>,
    code: String,
) -> Result<LockState, AppError> {
    let row = current_row(state.inner())?;
    if !lock::verify(&row, &code) {
        return Err(AppError::new("bad_lock_code", "口令不正确"));
    }
    let st = state.inner().clone();
    let mut next = st
        .lock
        .lock()
        .map(|guard| guard.clone())
        .map_err(|_| AppError::internal("锁状态不可用"))?;
    next.locked = false;
    st.update_lock(next);
    let snapshot = st.lock_state();
    emit(&st, snapshot.clone());
    Ok(snapshot)
}

#[tauri::command]
pub async fn app_lock_engage(state: State<'_, AppState>) -> Result<LockState, AppError> {
    let st = state.inner().clone();
    {
        let mut guard = st
            .lock
            .lock()
            .map_err(|_| AppError::internal("锁状态不可用"))?;
        if guard.enabled {
            guard.locked = true;
        }
    }
    let snapshot = st.lock_state();
    emit(&st, snapshot.clone());
    Ok(snapshot)
}