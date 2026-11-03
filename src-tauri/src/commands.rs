use crate::{
    db::{SettingsSnapshot, Theme, SCHEMA_VERSION},
    error::AppError,
    service::LocalService,
};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

pub const SETTINGS_EVENT: &str = "taskdock://settings-changed";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapSnapshot {
    theme: Theme,
    desktop: crate::desktop::settings::DesktopSettings,
    revision: i64,
    app_version: &'static str,
    schema_version: u32,
}

#[tauri::command]
pub async fn bootstrap(
    database: State<'_, Arc<LocalService>>,
) -> Result<BootstrapSnapshot, AppError> {
    let database = Arc::clone(database.inner());
    let settings = tauri::async_runtime::spawn_blocking(move || database.settings())
        .await
        .map_err(AppError::internal)?
        .map_err(AppError::service)?;
    Ok(BootstrapSnapshot {
        theme: settings.theme,
        desktop: settings.desktop,
        revision: settings.revision,
        app_version: env!("CARGO_PKG_VERSION"),
        schema_version: SCHEMA_VERSION,
    })
}

#[tauri::command]
pub async fn set_theme(
    app: AppHandle,
    window: WebviewWindow,
    database: State<'_, Arc<LocalService>>,
    theme: Theme,
) -> Result<SettingsSnapshot, AppError> {
    if window.label() != "main" {
        return Err(AppError::forbidden());
    }
    let database = Arc::clone(database.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let runtime = app.state::<crate::desktop::DesktopRuntime>();
        let _gate = runtime.gate.lock().map_err(AppError::internal)?;
        let settings = database.set_theme(theme).map_err(AppError::internal)?;
        crate::desktop::theme::apply(&app, settings.theme);
        if let Err(error) = app.emit(SETTINGS_EVENT, &settings) {
            // The transaction has committed. A missed event is repaired by bootstrap on focus.
            log::warn!("settings committed but event delivery failed: {error}");
        }
        Ok(settings)
    })
    .await
    .map_err(AppError::internal)?
}

fn can_show(caller: &str, target: &str) -> bool {
    matches!(
        (caller, target),
        ("main", "widget" | "quick-entry") | ("widget", "quick-entry")
    )
}

#[tauri::command]
pub async fn show_window(
    app: AppHandle,
    window: WebviewWindow,
    label: String,
) -> Result<(), AppError> {
    if !can_show(window.label(), &label) {
        return Err(AppError::forbidden());
    }
    tauri::async_runtime::spawn_blocking(move || {
        if label == "widget" {
            crate::desktop::show_widget(&app)
        } else {
            crate::desktop::present(&app, &label)
        }
    })
    .await
    .map_err(AppError::internal)?
}
#[tauri::command]
pub async fn hide_current_window(app: AppHandle, window: WebviewWindow) -> Result<(), AppError> {
    match window.label() {
        "widget" => tauri::async_runtime::spawn_blocking(move || crate::desktop::hide_widget(&app))
            .await
            .map_err(AppError::internal)?,
        "quick-entry" => window.hide().map_err(AppError::internal),
        _ => Err(AppError::forbidden()),
    }
}
#[tauri::command]
pub fn report_frontend_error(code: String) -> Result<(), AppError> {
    if !matches!(
        code.as_str(),
        "VUE_ERROR" | "UNHANDLED_REJECTION" | "WINDOW_ERROR" | "SERVICE_ERROR"
    ) {
        return Err(AppError::forbidden());
    }
    log::warn!("frontend: {code}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn window_targets_are_restricted() {
        assert!(can_show("main", "widget"));
        assert!(can_show("main", "quick-entry"));
        assert!(can_show("widget", "quick-entry"));
        for (caller, target) in [
            ("quick-entry", "main"),
            ("widget", "main"),
            ("widget", "widget"),
            ("main", "remote"),
            ("unknown", "widget"),
        ] {
            assert!(!can_show(caller, target));
        }
    }
}
