use super::{runtime, settings::DesktopSettings};
use crate::{db::SettingsSnapshot, error::AppError};
use tauri::{AppHandle, WebviewWindow};
#[tauri::command]
pub fn desktop_status(app: AppHandle) -> runtime::RuntimeStatus {
    runtime::status(&app)
}
#[tauri::command]
pub async fn save_desktop_settings(
    app: AppHandle,
    window: WebviewWindow,
    settings: DesktopSettings,
    expected_revision: i64,
) -> Result<SettingsSnapshot, AppError> {
    if window.label() != "main" {
        return Err(AppError::forbidden());
    }
    tauri::async_runtime::spawn_blocking(move || runtime::save(&app, settings, expected_revision))
        .await
        .map_err(AppError::internal)?
}
#[tauri::command]
pub async fn reset_window_positions(app: AppHandle, window: WebviewWindow) -> Result<(), AppError> {
    if window.label() != "main" {
        return Err(AppError::forbidden());
    }
    tauri::async_runtime::spawn_blocking(move || runtime::restore(&app, true))
        .await
        .map_err(AppError::internal)?
}
#[tauri::command]
pub fn exit_app(app: AppHandle, window: WebviewWindow) -> Result<(), AppError> {
    if window.label() != "main" {
        return Err(AppError::forbidden());
    }
    runtime::shutdown(&app);
    Ok(())
}
#[tauri::command]
pub async fn set_widget_layout(
    app: AppHandle,
    window: WebviewWindow,
    expanded: bool,
    width: f64,
    height: f64,
) -> Result<(), AppError> {
    if window.label() != "widget" {
        return Err(AppError::forbidden());
    }
    tauri::async_runtime::spawn_blocking(move || {
        runtime::set_widget_layout(&app, expanded, width, height)
    })
    .await
    .map_err(AppError::internal)?
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WidgetAction {
    Drag,
    Resize,
}
// Pointer gestures run synchronously on the event-loop thread, matching Tauri's
// native window commands. Scheduling them as async work can miss mouse-down.
#[tauri::command]
pub fn widget_window_action(window: WebviewWindow, action: WidgetAction) -> Result<(), AppError> {
    if window.label() != "widget" {
        return Err(AppError::forbidden());
    }
    match action {
        WidgetAction::Drag => window.start_dragging().map_err(AppError::internal),
        WidgetAction::Resize => window
            .as_ref()
            .window()
            .start_resize_dragging(tauri_runtime::ResizeDirection::SouthEast)
            .map_err(AppError::internal),
    }
}
#[tauri::command]
pub async fn lock_widget(app: AppHandle, window: WebviewWindow) -> Result<(), AppError> {
    if window.label() != "widget" {
        return Err(AppError::forbidden());
    }
    tauri::async_runtime::spawn_blocking(move || runtime::set_widget_locked(&app, true))
        .await
        .map_err(AppError::internal)?
}
pub fn show_widget(app: &AppHandle) -> Result<(), AppError> {
    runtime::visibility(app, true)?;
    runtime::present(app, "widget")
}
pub fn hide_widget(app: &AppHandle) -> Result<(), AppError> {
    runtime::visibility(app, false)
}
