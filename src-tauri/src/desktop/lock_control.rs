//! A button-sized owned window remains clickable while the widget itself is
//! genuinely transparent to mouse input, including clicks into other processes.
use super::{native, settings::DesktopSettings};
use crate::{error::AppError, service::LocalService};
use serde::Deserialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Bounds {
    pub fn valid(&self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|v| v.is_finite())
            && (0.0..=600.0).contains(&self.x)
            && (0.0..=800.0).contains(&self.y)
            && (16.0..=32.0).contains(&self.width)
            && (16.0..=32.0).contains(&self.height)
    }
}
#[derive(Default)]
struct Control {
    ready: AtomicBool,
    bounds: Mutex<Option<Bounds>>,
}
pub fn create(app: &AppHandle, data_directory: std::path::PathBuf) -> tauri::Result<()> {
    let owner = app
        .get_webview_window("widget")
        .expect("widget must be created first");
    let builder = WebviewWindowBuilder::new(
        app,
        "widget-lock",
        WebviewUrl::App("widget-lock.html".into()),
    )
    .title("TaskDock · 解锁")
    .inner_size(22.0, 22.0)
    .decorations(false)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .skip_taskbar(true)
    .focused(false)
    .focusable(false)
    .shadow(false)
    .visible(false)
    .data_directory(data_directory);
    #[cfg(windows)]
    let builder = builder.owner(&owner)?;
    #[cfg(not(windows))]
    let builder = builder.parent(&owner)?;
    builder.build()?;
    Ok(())
}
pub fn initialize(app: &AppHandle) {
    app.manage(Control::default());
}
pub fn set_bounds(app: &AppHandle, bounds: Bounds) -> Result<(), AppError> {
    if !bounds.valid() {
        return Err(AppError::internal("invalid widget lock bounds"));
    }
    *app.state::<Control>()
        .bounds
        .lock()
        .map_err(AppError::internal)? = Some(bounds);
    Ok(())
}
pub fn ready(app: &AppHandle) {
    app.state::<Control>().ready.store(true, Ordering::SeqCst);
}
/// Caller holds the desktop gate and runs off the UI thread.
pub fn sync(app: &AppHandle, prefs: &DesktopSettings) -> Result<(), AppError> {
    let Some(control) = app.try_state::<Control>() else {
        return Ok(());
    };
    let Some(window) = app.get_webview_window("widget-lock") else {
        return Ok(());
    };
    let owner = app
        .get_webview_window("widget")
        .ok_or_else(|| AppError::internal("widget missing"))?;
    let bounds = *control.bounds.lock().map_err(AppError::internal)?;
    let Some(bounds) = bounds.filter(|_| {
        prefs.widget_locked && prefs.widget_visible && control.ready.load(Ordering::SeqCst)
    }) else {
        return window.hide().map_err(AppError::internal);
    };
    let scale = owner.scale_factor().map_err(AppError::internal)?;
    let size = owner.inner_size().map_err(AppError::internal)?;
    if !owner.is_visible().map_err(AppError::internal)?
        || owner.is_minimized().map_err(AppError::internal)?
        || bounds.x + bounds.width > size.width as f64 / scale + 1.0
        || bounds.y + bounds.height > size.height as f64 / scale + 1.0
    {
        return window.hide().map_err(AppError::internal);
    }
    let origin = owner.inner_position().map_err(AppError::internal)?;
    window
        .set_size(PhysicalSize::new(
            (bounds.width * scale).round() as u32,
            (bounds.height * scale).round() as u32,
        ))
        .map_err(AppError::internal)?;
    window
        .set_position(PhysicalPosition::new(
            origin.x + (bounds.x * scale).round() as i32,
            origin.y + (bounds.y * scale).round() as i32,
        ))
        .map_err(AppError::internal)?;
    window
        .set_always_on_top(prefs.widget_topmost)
        .map_err(AppError::internal)?;
    window.show().map_err(AppError::internal)?;
    native::lock_control_style(&window).map_err(AppError::internal)
}
pub fn sync_current(app: &AppHandle) -> Result<(), AppError> {
    let prefs = app
        .state::<Arc<LocalService>>()
        .settings()
        .map_err(AppError::service)?;
    sync(app, &prefs.desktop)
}
