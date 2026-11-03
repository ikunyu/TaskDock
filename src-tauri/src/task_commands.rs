use crate::{
    db::Database,
    error::{AppError, ServiceError},
    service::LocalService,
    tasks::*,
};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State, WebviewWindow};

const TASKS_EVENT: &str = "taskdock://tasks-changed";
#[derive(Clone, Serialize)]
struct TaskChange {
    revision: i64,
}

pub(crate) async fn run<R: Send + 'static>(
    service: &Arc<LocalService>,
    operation: impl FnOnce(&Database) -> Result<R, ServiceError> + Send + 'static,
) -> Result<R, AppError> {
    let service = Arc::clone(service);
    tauri::async_runtime::spawn_blocking(move || service.with_database(operation))
        .await
        .map_err(AppError::internal)?
        .map_err(AppError::service)
}
pub(crate) fn publish(app: &AppHandle, result: MutationResult) -> MutationResult {
    if let Err(error) = app.emit(
        TASKS_EVENT,
        TaskChange {
            revision: result.snapshot.revision,
        },
    ) {
        log::warn!("task transaction committed but notification failed: {error}");
    }
    result
}
fn authorize(window: &WebviewWindow, allowed: &[&str]) -> Result<(), AppError> {
    if allowed.contains(&window.label()) {
        Ok(())
    } else {
        Err(AppError::forbidden())
    }
}

#[tauri::command]
pub async fn tasks_snapshot(
    database: State<'_, Arc<LocalService>>,
) -> Result<TaskSnapshot, AppError> {
    run(database.inner(), Database::task_snapshot).await
}

#[tauri::command]
pub async fn create_task(
    app: AppHandle,
    window: WebviewWindow,
    database: State<'_, Arc<LocalService>>,
    draft: TaskDraft,
) -> Result<MutationResult, AppError> {
    authorize(&window, &["main", "quick-entry"])?;
    Ok(publish(
        &app,
        run(database.inner(), move |db| {
            validation::creation_dates(&draft, chrono::Local::now().date_naive())?;
            db.create_task(draft)
        })
        .await?,
    ))
}

#[tauri::command]
pub async fn update_task(
    app: AppHandle,
    window: WebviewWindow,
    database: State<'_, Arc<LocalService>>,
    id: String,
    expected_version: i64,
    draft: TaskDraft,
) -> Result<MutationResult, AppError> {
    authorize(&window, &["main"])?;
    Ok(publish(
        &app,
        run(database.inner(), move |db| {
            db.update_task(&id, expected_version, draft)
        })
        .await?,
    ))
}

#[tauri::command]
pub async fn set_task_status(
    app: AppHandle,
    window: WebviewWindow,
    database: State<'_, Arc<LocalService>>,
    id: String,
    expected_version: i64,
    status: TaskStatus,
    confirm_incomplete: bool,
) -> Result<MutationResult, AppError> {
    authorize(&window, &["main", "widget"])?;
    Ok(publish(
        &app,
        run(database.inner(), move |db| {
            db.set_task_status(&id, expected_version, status, confirm_incomplete)
        })
        .await?,
    ))
}

#[tauri::command]
pub async fn purge_task(
    app: AppHandle,
    window: WebviewWindow,
    database: State<'_, Arc<LocalService>>,
    id: String,
    expected_version: i64,
    confirmed: bool,
) -> Result<MutationResult, AppError> {
    authorize(&window, &["main"])?;
    Ok(publish(
        &app,
        run(database.inner(), move |db| {
            db.purge_task(&id, expected_version, confirmed)
        })
        .await?,
    ))
}

#[tauri::command]
pub async fn save_category(
    app: AppHandle,
    window: WebviewWindow,
    database: State<'_, Arc<LocalService>>,
    draft: CategoryDraft,
) -> Result<MutationResult, AppError> {
    authorize(&window, &["main"])?;
    Ok(publish(
        &app,
        run(database.inner(), move |db| db.save_category(draft)).await?,
    ))
}

#[tauri::command]
pub async fn delete_category(
    app: AppHandle,
    window: WebviewWindow,
    database: State<'_, Arc<LocalService>>,
    id: String,
    expected_version: i64,
) -> Result<MutationResult, AppError> {
    authorize(&window, &["main"])?;
    Ok(publish(
        &app,
        run(database.inner(), move |db| {
            db.delete_category(&id, expected_version)
        })
        .await?,
    ))
}
