use crate::{error::AppError, service::LocalService, tasks::MutationResult};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State, WebviewWindow};

#[derive(Default)]
pub struct OpenTask(pub Mutex<Option<String>>);

#[tauri::command]
pub async fn reminder_action(
    app: AppHandle,
    window: WebviewWindow,
    database: State<'_, Arc<LocalService>>,
    id: String,
    minutes: Option<i64>,
) -> Result<MutationResult, AppError> {
    if window.label() != "main" {
        return Err(AppError::forbidden());
    }
    let result =
        crate::task_commands::run(database.inner(), move |db| db.reminder_action(&id, minutes))
            .await?;
    Ok(crate::task_commands::publish(&app, result))
}
#[tauri::command]
pub fn take_notification_task(
    window: WebviewWindow,
    pending: State<'_, OpenTask>,
) -> Result<Option<String>, AppError> {
    if window.label() != "main" {
        return Err(AppError::forbidden());
    }
    Ok(pending.0.lock().map_err(AppError::internal)?.take())
}
