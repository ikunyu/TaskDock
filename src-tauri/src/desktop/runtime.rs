use super::{
    geometry::{self, WorkArea},
    native,
    settings::{DesktopSettings, Placement},
};
use crate::{
    commands::SETTINGS_EVENT, db::SettingsSnapshot, error::AppError, service::LocalService,
};
use serde::Serialize;
use std::{
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub ready: bool,
    pub effective_shortcut: Option<String>,
    pub autostart_enabled: Option<bool>,
    pub warnings: Vec<String>,
}
#[derive(Default)]
pub struct DesktopRuntime {
    pub gate: Mutex<()>,
    pub status: Mutex<RuntimeStatus>,
    pub quitting: AtomicBool,
    pub restoring: AtomicBool,
    widget_locked: AtomicBool,
    widget_layout: Mutex<WidgetLayout>,
}
#[derive(Clone)]
struct WidgetLayout {
    expanded: bool,
    expanded_size: (f64, f64),
    compact_size: (f64, f64),
}
impl Default for WidgetLayout {
    fn default() -> Self {
        Self {
            expanded: true,
            expanded_size: (180.0, 240.0),
            compact_size: (160.0, 54.0),
        }
    }
}
fn apply_widget_layout(
    w: &tauri::WebviewWindow,
    layout: &WidgetLayout,
    at: Option<(i32, i32)>,
) -> Result<(), AppError> {
    if layout.expanded {
        w.set_max_size(Some(LogicalSize::new(600.0, 800.0)))
            .map_err(internal)?;
        w.set_min_size(Some(LogicalSize::new(160.0, 120.0)))
            .map_err(internal)?;
        w.set_size(LogicalSize::new(
            layout.expanded_size.0,
            layout.expanded_size.1,
        ))
        .map_err(internal)?;
        w.set_resizable(true).map_err(internal)?;
        if let Some((x, y)) = at {
            w.set_position(PhysicalPosition::new(x, y))
                .map_err(internal)?;
        }
    } else {
        let current = w.outer_position().map_err(internal)?;
        let (x, y) = at.unwrap_or((current.x, current.y));
        let p = Placement {
            label: "widget".into(),
            x,
            y,
            width: layout.compact_size.0,
            height: layout.compact_size.1,
            scale: w.scale_factor().map_err(internal)?,
        };
        let safe = geometry::fit_compact(&p, &areas(w.app_handle())?).unwrap_or(p);
        w.set_min_size(Some(LogicalSize::new(
            120.0_f64.min(safe.width),
            44.0_f64.min(safe.height),
        )))
        .map_err(internal)?;
        w.set_max_size(Some(LogicalSize::new(480.0, 600.0)))
            .map_err(internal)?;
        w.set_size(LogicalSize::new(safe.width, safe.height))
            .map_err(internal)?;
        w.set_resizable(false).map_err(internal)?;
        w.set_position(PhysicalPosition::new(safe.x, safe.y))
            .map_err(internal)?;
    }
    Ok(())
}
pub fn set_widget_layout(
    app: &AppHandle,
    expanded: bool,
    width: f64,
    height: f64,
) -> Result<(), AppError> {
    if !width.is_finite()
        || !(120.0..=480.0).contains(&width)
        || !height.is_finite()
        || !(44.0..=600.0).contains(&height)
    {
        return Err(AppError::service(crate::error::ServiceError::Validation(
            "悬浮窗内容尺寸无效。",
        )));
    }
    let rt = app.state::<DesktopRuntime>();
    let _gate = rt.gate.lock().map_err(internal)?;
    let mut layout = rt.widget_layout.lock().map_err(internal)?;
    let w = app
        .get_webview_window("widget")
        .ok_or_else(|| internal("widget missing"))?;
    let position = w.outer_position().map_err(internal)?;
    let mut previous = layout.clone();
    if layout.expanded {
        let scale = w.scale_factor().map_err(internal)?;
        let size = w.inner_size().map_err(internal)?;
        previous.expanded_size = (
            (size.width as f64 / scale).clamp(160.0, 600.0),
            (size.height as f64 / scale).clamp(120.0, 800.0),
        );
    }
    let mut next = previous.clone();
    next.expanded = expanded;
    next.compact_size = (width, height);
    if expanded {
        // Fit content height without changing the current/user-selected width.
        next.expanded_size.1 = height.clamp(120.0, 800.0);
    }
    let safe = if expanded {
        let p = Placement {
            label: "widget".into(),
            x: position.x,
            y: position.y,
            width: next.expanded_size.0,
            height: next.expanded_size.1,
            scale: w.scale_factor().map_err(internal)?,
        };
        geometry::fit("widget", Some(&p), &areas(app)?)
    } else {
        None
    };
    if let Some(p) = &safe {
        next.expanded_size = (p.width, p.height);
    }
    let at = safe.as_ref().map(|p| (p.x, p.y));
    let prefs = service(app).settings().map_err(AppError::service)?;
    let operation = apply_widget_layout(&w, &next, at).and_then(|_| {
        native::widget_style(
            &w,
            prefs.desktop.widget_opacity,
            prefs.desktop.widget_locked,
        )
        .map_err(internal)
    });
    if let Err(error) = operation {
        let _ = apply_widget_layout(&w, &previous, Some((position.x, position.y)));
        let _ = native::widget_style(
            &w,
            prefs.desktop.widget_opacity,
            prefs.desktop.widget_locked,
        );
        *layout = previous;
        return Err(error);
    }
    *layout = next;
    Ok(())
}
fn internal(e: impl std::fmt::Display) -> AppError {
    AppError::internal(e)
}
fn service(app: &AppHandle) -> Arc<LocalService> {
    app.state::<Arc<LocalService>>().inner().clone()
}
pub fn status(app: &AppHandle) -> RuntimeStatus {
    app.state::<DesktopRuntime>()
        .status
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default()
}
pub(super) fn warn(app: &AppHandle, message: &str) {
    log::warn!("desktop: {message}");
    if let Ok(mut s) = app.state::<DesktopRuntime>().status.lock() {
        if !s.warnings.iter().any(|m| m == message) {
            s.warnings.push(message.into());
        }
    }
    let _ = app.emit("taskdock://desktop-status", status(app));
}
pub fn areas(app: &AppHandle) -> Result<Vec<WorkArea>, AppError> {
    let primary = app
        .primary_monitor()
        .map_err(internal)?
        .map(|m| *m.position());
    Ok(app
        .available_monitors()
        .map_err(internal)?
        .into_iter()
        .map(|m| {
            let r = m.work_area();
            WorkArea {
                x: r.position.x,
                y: r.position.y,
                width: r.size.width,
                height: r.size.height,
                scale: m.scale_factor(),
                primary: primary == Some(*m.position()),
            }
        })
        .collect())
}
fn placement(window: &tauri::WebviewWindow) -> Option<Placement> {
    let rt = window.app_handle().state::<DesktopRuntime>();
    let layout = if window.label() == "widget" {
        Some(rt.widget_layout.lock().ok()?)
    } else {
        None
    };
    if window.is_minimized().ok()? || window.is_maximized().ok()? {
        return None;
    }
    let p = window.outer_position().ok()?;
    let size = window.inner_size().ok()?;
    let scale = window.scale_factor().ok()?;
    let (width, height) = match layout.as_ref() {
        Some(l) if !l.expanded => l.expanded_size,
        _ => (size.width as f64 / scale, size.height as f64 / scale),
    };
    let p = Placement {
        label: window.label().into(),
        x: p.x,
        y: p.y,
        width,
        height,
        scale,
    };
    p.valid().then_some(p)
}
fn apply_placement(app: &AppHandle, p: &Placement) -> Result<(), AppError> {
    let w = app
        .get_webview_window(&p.label)
        .ok_or_else(|| internal("configured window missing"))?;
    if p.label == "widget" {
        let rt = app.state::<DesktopRuntime>();
        let mut layout = rt.widget_layout.lock().map_err(internal)?;
        layout.expanded_size = (p.width, p.height);
        return apply_widget_layout(&w, &layout, Some((p.x, p.y)));
    } else {
        w.set_size(LogicalSize::new(p.width, p.height))
            .map_err(internal)?;
    }
    w.set_position(PhysicalPosition::new(p.x, p.y))
        .map_err(internal)
}
pub fn restore(app: &AppHandle, reset: bool) -> Result<(), AppError> {
    let runtime = app.state::<DesktopRuntime>();
    runtime.restoring.store(true, Ordering::SeqCst);
    let result = (|| {
        let monitors = areas(app)?;
        let mut saved = if reset {
            vec![]
        } else {
            service(app)
                .with_database(|db| db.placements())
                .map_err(AppError::service)?
        };
        // Adopt the narrower default for previous default/minimum widths;
        // preserve other user-selected sizes and never alter task data.
        for placement in &mut saved {
            if placement.label == "widget"
                && [200.0, 220.0, 240.0, 320.0]
                    .iter()
                    .any(|width| (placement.width - width).abs() < 0.5)
            {
                placement.width = 180.0;
            }
        }
        for label in ["main", "widget", "quick-entry"] {
            if let Some(p) =
                geometry::fit(label, saved.iter().find(|p| p.label == label), &monitors)
            {
                apply_placement(app, &p)?;
            }
        }
        let prefs = service(app).settings().map_err(AppError::service)?;
        let w = app
            .get_webview_window("widget")
            .ok_or_else(|| internal("widget missing"))?;
        native::widget_style(
            &w,
            prefs.desktop.widget_opacity,
            prefs.desktop.widget_locked,
        )
        .map_err(internal)?;
        capture(app)
    })();
    runtime.restoring.store(false, Ordering::SeqCst);
    result
}
fn capture(app: &AppHandle) -> Result<(), AppError> {
    let placements: Vec<_> = ["main", "widget", "quick-entry"]
        .iter()
        .filter_map(|label| app.get_webview_window(label).and_then(|w| placement(&w)))
        .collect();
    service(app)
        .with_database(|db| db.save_placements(&placements))
        .map_err(AppError::service)
}
pub fn present(app: &AppHandle, label: &str) -> Result<(), AppError> {
    if !matches!(label, "main" | "widget" | "quick-entry") {
        return Err(AppError::forbidden());
    }
    // Widget presentation is called by the existing blocking show-window worker.
    let rt = app.state::<DesktopRuntime>();
    let _gate = if label == "widget" {
        Some(rt.gate.lock().map_err(internal)?)
    } else {
        None
    };
    let w = app
        .get_webview_window(label)
        .ok_or_else(|| internal("configured window missing"))?;
    let saved = placement(&w);
    if let Some(p) = geometry::fit(label, saved.as_ref(), &areas(app)?) {
        apply_placement(app, &p)?;
    }
    w.unminimize().map_err(internal)?;
    w.show().map_err(internal)?;
    if label == "widget" {
        let prefs = service(app).settings().map_err(AppError::service)?;
        native::widget_style(
            &w,
            prefs.desktop.widget_opacity,
            prefs.desktop.widget_locked,
        )
        .map_err(internal)?;
    }
    if label != "widget"
        || !app
            .state::<DesktopRuntime>()
            .widget_locked
            .load(Ordering::SeqCst)
    {
        w.set_focus().map_err(internal)?;
    }
    if label == "quick-entry" {
        let _ = w.emit("taskdock://quick-focus", ());
    }
    Ok(())
}
fn apply_widget(app: &AppHandle, p: &DesktopSettings) -> Result<(), AppError> {
    let w = app
        .get_webview_window("widget")
        .ok_or_else(|| internal("widget missing"))?;
    w.set_always_on_top(p.widget_topmost).map_err(internal)?;
    if p.widget_visible {
        w.show().map_err(internal)?;
    } else {
        w.hide().map_err(internal)?;
    }
    native::widget_style(&w, p.widget_opacity, p.widget_locked).map_err(internal)?;
    app.state::<DesktopRuntime>()
        .widget_locked
        .store(p.widget_locked, Ordering::SeqCst);
    Ok(())
}
fn set_autostart(app: &AppHandle, enabled: bool) -> Result<(), AppError> {
    if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|e| {
        log::warn!("autostart change failed: {e}");
        AppError {
            code: "AUTOSTART_FAILED",
            message: "开机启动注册未完成，请重试。原设置已保留。",
        }
    })
}
/// Caller holds DesktopRuntime.gate. Reapply OS resources without spawning
/// another tray listener, geometry worker, shortcut handler, or scheduler.
pub(crate) fn reload_after_restore(app: &AppHandle) -> Vec<String> {
    let mut warnings = vec![];
    let result = (|| {
        let prefs = service(app).settings().map_err(AppError::service)?;
        super::theme::apply(app, prefs.theme);
        let old = status(app).effective_shortcut;
        let lease = super::shortcut::ShortcutLease::prepare(
            old.as_deref(),
            &prefs.desktop.shortcut,
            |key| app.global_shortcut().register(key).map_err(shortcut_error),
            |key| app.global_shortcut().unregister(key).map_err(internal),
        );
        match lease {
            Ok(lease) => {
                lease.commit();
                if old.as_deref() != Some(&prefs.desktop.shortcut) {
                    if let Some(k) = old {
                        if let Err(e) = app.global_shortcut().unregister(k.as_str()) {
                            log::warn!("restore shortcut cleanup: {e}");
                        }
                    }
                }
                if let Ok(mut s) = app.state::<DesktopRuntime>().status.lock() {
                    s.effective_shortcut = Some(prefs.desktop.shortcut.clone());
                }
            }
            Err(e) => warnings.push(e.message.to_string()),
        }
        if let Err(e) = set_autostart(app, prefs.desktop.autostart) {
            warnings.push(e.message.to_string());
        }
        if let Err(e) = apply_widget(app, &prefs.desktop) {
            warnings.push(e.message.to_string());
        }
        if let Err(e) = restore(app, false) {
            warnings.push(e.message.to_string());
        }
        let _ = app.emit(SETTINGS_EVENT, &prefs);
        Ok::<_, AppError>(())
    })();
    if let Err(e) = result {
        warnings.push(e.message.into());
    }
    if let Ok(mut status) = app.state::<DesktopRuntime>().status.lock() {
        status.warnings = warnings.clone();
        status.autostart_enabled = app.autolaunch().is_enabled().ok();
    }
    let _ = app.emit("taskdock://desktop-status", status(app));
    warnings
}
fn shortcut_error(e: impl std::fmt::Display) -> AppError {
    log::warn!("shortcut registration failed: {e}");
    AppError {
        code: "SHORTCUT_CONFLICT",
        message: "快捷键被占用或无法注册，请换一个组合。原快捷键设置未更改。",
    }
}
/// Keep the old registration until persistence succeeds. The event handler only
/// honors effective_shortcut, so a failed cleanup cannot activate an obsolete key.
pub fn save(
    app: &AppHandle,
    mut next: DesktopSettings,
    expected: i64,
) -> Result<SettingsSnapshot, AppError> {
    next.validate().map_err(AppError::service)?;
    let rt = app.state::<DesktopRuntime>();
    let _gate = rt.gate.lock().map_err(internal)?;
    if !status(app).ready {
        return Err(AppError {
            code: "DESKTOP_STARTING",
            message: "桌面服务正在初始化，请稍后重试。",
        });
    }
    let db = service(app);
    let old = db.settings().map_err(AppError::service)?;
    if old.revision != expected {
        return Err(AppError::service(crate::error::ServiceError::Conflict));
    }
    let old_key = status(app).effective_shortcut;
    let changed = old.desktop.shortcut != next.shortcut;
    let lease = super::shortcut::ShortcutLease::for_settings(
        &old.desktop.shortcut,
        old_key.as_deref(),
        &next.shortcut,
        |key| app.global_shortcut().register(key).map_err(shortcut_error),
        |key| app.global_shortcut().unregister(key).map_err(internal),
    )?;
    let actual_auto = app.autolaunch().is_enabled().map_err(internal)?;
    let operation = (|| {
        if next.autostart != actual_auto {
            set_autostart(app, next.autostart)?;
        }
        apply_widget(app, &next)?;
        db.with_database(|db| db.save_desktop(next.clone(), expected))
            .map_err(AppError::service)
    })();
    match operation {
        Ok(snapshot) => {
            if let Some(lease) = lease {
                lease.commit();
            }
            if let Ok(mut state) = rt.status.lock() {
                if changed {
                    state.effective_shortcut = Some(next.shortcut.clone());
                    state.warnings.retain(|message| !message.contains("快捷键"));
                }
                state.autostart_enabled = Some(next.autostart);
            }
            if changed && old_key.as_deref() != Some(next.shortcut.as_str()) {
                if let Some(key) = old_key {
                    if let Err(e) = app.global_shortcut().unregister(key.as_str()) {
                        warn(app, "旧快捷键清理失败，旧键已在事件处理层禁用。");
                        log::warn!("shortcut cleanup: {e}");
                    }
                }
            }
            let _ = app.emit(SETTINGS_EVENT, &snapshot);
            let _ = app.emit("taskdock://desktop-status", status(app));
            Ok(snapshot)
        }
        Err(error) => {
            if next.autostart != actual_auto && set_autostart(app, actual_auto).is_err() {
                warn(app, "启动项补偿失败，请检查设置中的实际状态。");
            }
            if apply_widget(app, &old.desktop).is_err() {
                warn(app, "窗口设置补偿失败，请重新应用设置。");
            }
            if let Ok(mut state) = rt.status.lock() {
                state.autostart_enabled = app.autolaunch().is_enabled().ok();
            }
            Err(error)
        }
    }
}
pub fn visibility(app: &AppHandle, visible: bool) -> Result<(), AppError> {
    let rt = app.state::<DesktopRuntime>();
    let _gate = rt.gate.lock().map_err(internal)?;
    if !status(app).ready {
        return Err(AppError {
            code: "DESKTOP_STARTING",
            message: "桌面服务正在初始化，请稍后重试。",
        });
    }
    let w = app
        .get_webview_window("widget")
        .ok_or_else(|| internal("widget missing"))?;
    let previous = w.is_visible().map_err(internal)?;
    let prefs = service(app).settings().map_err(AppError::service)?;
    let result = (|| {
        if visible {
            w.show().map_err(internal)?;
        } else {
            w.hide().map_err(internal)?;
        }
        native::widget_style(
            &w,
            prefs.desktop.widget_opacity,
            prefs.desktop.widget_locked,
        )
        .map_err(internal)?;
        service(app)
            .with_database(|db| db.set_widget_visibility(visible))
            .map_err(AppError::service)
    })();
    let snapshot = match result {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let rollback = if previous { w.show() } else { w.hide() };
            if let Err(e) = rollback {
                log::warn!("widget visibility rollback failed: {e}");
            }
            let _ = native::widget_style(
                &w,
                prefs.desktop.widget_opacity,
                prefs.desktop.widget_locked,
            );
            return Err(error);
        }
    };
    let _ = app.emit(SETTINGS_EVENT, &snapshot);
    Ok(())
}
pub fn set_widget_locked(app: &AppHandle, locked: bool) -> Result<(), AppError> {
    let rt = app.state::<DesktopRuntime>();
    let _gate = rt.gate.lock().map_err(internal)?;
    if !status(app).ready {
        return Err(AppError {
            code: "DESKTOP_STARTING",
            message: "桌面服务正在初始化，请稍后重试。",
        });
    }
    let old = service(app).settings().map_err(AppError::service)?;
    let w = app
        .get_webview_window("widget")
        .ok_or_else(|| internal("widget missing"))?;
    let result = (|| {
        native::widget_style(&w, old.desktop.widget_opacity, locked).map_err(internal)?;
        service(app)
            .with_database(|db| db.set_widget_locked(locked))
            .map_err(AppError::service)
    })();
    let snapshot = match result {
        Ok(snapshot) => snapshot,
        Err(error) => {
            if let Err(e) =
                native::widget_style(&w, old.desktop.widget_opacity, old.desktop.widget_locked)
            {
                log::warn!("widget lock rollback failed: {e}");
                warn(app, "悬浮窗锁定状态恢复失败，请从托盘解锁后重试。");
            }
            return Err(error);
        }
    };
    rt.widget_locked.store(locked, Ordering::SeqCst);
    let _ = app.emit(SETTINGS_EVENT, &snapshot);
    Ok(())
}
pub fn shortcut_pressed(app: &AppHandle, shortcut: &Shortcut) {
    let active = status(app)
        .effective_shortcut
        .and_then(|s| Shortcut::from_str(&s).ok());
    if active.as_ref() == Some(shortcut) {
        if let Err(e) = present(app, "quick-entry") {
            warn(app, e.message);
        }
    }
}
pub fn initialize(app: AppHandle, background: bool) {
    std::thread::spawn(move || {
        let rt = app.state::<DesktopRuntime>();
        let settings = {
            // Serialize the initial native theme with later user changes and restore.
            let _gate = match rt.gate.lock() {
                Ok(gate) => gate,
                Err(e) => {
                    log::error!("desktop initialization gate: {e}");
                    let _ = present(&app, "main");
                    return;
                }
            };
            let settings = match service(&app).settings() {
                Ok(v) => v,
                Err(e) => {
                    log::error!("desktop initialization: {e}");
                    let _ = present(&app, "main");
                    return;
                }
            };
            super::theme::apply(&app, settings.theme);
            settings
        };
        if let Err(e) = restore(&app, false) {
            warn(&app, e.message);
        }
        if let Err(e) = apply_widget(&app, &settings.desktop) {
            warn(&app, e.message);
        }
        // Moving the portable/install folder must not leave an existing,
        // explicitly enabled startup entry pointing at the old executable.
        if settings.desktop.autostart && app.autolaunch().is_enabled().unwrap_or(false) {
            if let Err(error) = app.autolaunch().enable() {
                log::warn!("existing autostart path refresh failed: {error}");
                warn(
                    &app,
                    "开机启动路径更新失败，请在设置中重新保存开机启动选项。",
                );
            }
        }
        let key = settings.desktop.shortcut.clone();
        match app.global_shortcut().register(key.as_str()) {
            Ok(()) => {
                if let Ok(mut s) = rt.status.lock() {
                    s.effective_shortcut = Some(key);
                }
            }
            Err(e) => {
                log::warn!("saved shortcut unavailable: {e}");
                warn(
                    &app,
                    "快捷键暂不可用，任务和悬浮窗仍可使用。可在设置中更换组合。",
                );
            }
        }
        if let Ok(mut s) = rt.status.lock() {
            s.ready = true;
            s.autostart_enabled = app.autolaunch().is_enabled().ok();
        }
        let _ = app.emit("taskdock://desktop-status", status(&app));
        if !background {
            let _ = present(&app, "main");
        }
        let mut last = Vec::new();
        let mut layout = areas(&app).unwrap_or_default();
        while !rt.quitting.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(750));
            if rt.quitting.load(Ordering::SeqCst) {
                break;
            }
            let current = areas(&app).unwrap_or_else(|_| layout.clone());
            if current != layout {
                for label in ["main", "widget", "quick-entry"] {
                    if let Some(w) = app.get_webview_window(label) {
                        let p = placement(&w);
                        if let Some(safe) = geometry::fit(label, p.as_ref(), &current) {
                            let _ = apply_placement(&app, &safe);
                        }
                    }
                }
                layout = current;
            }
            if rt.restoring.load(Ordering::SeqCst) {
                continue;
            }
            let positions: Vec<_> = ["main", "widget", "quick-entry"]
                .iter()
                .filter_map(|l| app.get_webview_window(l).and_then(|w| placement(&w)))
                .collect();
            if positions != last {
                match service(&app).with_database(|db| db.save_placements(&positions)) {
                    Ok(()) => last = positions,
                    Err(e) => log::warn!("window state persistence failed: {e}"),
                }
            }
        }
    });
}
/// Drain the existing service before the verified installer can replace files.
pub(crate) fn prepare_update(app: &AppHandle) -> Result<(), AppError> {
    let rt = app.state::<DesktopRuntime>();
    let _gate = rt.gate.lock().map_err(internal)?;
    if rt.quitting.load(Ordering::SeqCst) {
        return Err(internal("application already quitting"));
    }
    capture(app)?;
    service(app).shutdown().map_err(AppError::service)
}
pub fn shutdown(app: &AppHandle) {
    if app
        .state::<DesktopRuntime>()
        .quitting
        .swap(true, Ordering::SeqCst)
    {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let _gate = app.state::<DesktopRuntime>();
        let _guard = _gate.gate.lock();
        if let Err(e) = capture(&app) {
            log::warn!("final window state: {}", e.code);
        }
        if let Err(e) = app.global_shortcut().unregister_all() {
            log::warn!("shortcut shutdown: {e}");
        }
        if let Err(e) = service(&app).shutdown() {
            log::warn!("database shutdown: {e}");
        }
        app.exit(0);
    });
}
pub fn window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let app = window.app_handle();
        if window.label() == "main" {
            let close = service(app)
                .settings()
                .map(|s| s.desktop.close_behavior)
                .unwrap_or_else(|_| "exit".into());
            if close == "exit" {
                shutdown(app);
            } else {
                let _ = window.hide();
            }
        } else if window.label() == "widget" {
            let app = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                if let Err(e) = visibility(&app, false) {
                    warn(&app, e.message);
                }
            });
        } else {
            let _ = window.hide();
        }
    }
}
pub fn install_tray(app: &AppHandle) -> tauri::Result<()> {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    };
    let main = MenuItem::with_id(app, "main", "打开主窗口", true, None::<&str>)?;
    let quick = MenuItem::with_id(app, "quick", "快速添加任务", true, None::<&str>)?;
    let widget = MenuItem::with_id(app, "widget", "显示/隐藏悬浮窗", true, None::<&str>)?;
    let unlock = MenuItem::with_id(app, "unlock-widget", "解锁悬浮窗", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "打开设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 TaskDock", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&main, &quick, &widget, &unlock, &settings, &quit])?;
    let mut tray = TrayIconBuilder::with_id("taskdock")
        .tooltip("TaskDock · 本地待办")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "main" => {
                let _ = present(app, "main");
            }
            "quick" => {
                let _ = present(app, "quick-entry");
            }
            "widget" => {
                let app = app.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    let visible = app
                        .get_webview_window("widget")
                        .and_then(|w| w.is_visible().ok())
                        .unwrap_or(false);
                    if let Err(e) = visibility(&app, !visible) {
                        warn(&app, e.message);
                    }
                });
            }
            "unlock-widget" => {
                let app = app.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    if let Err(e) = set_widget_locked(&app, false) {
                        warn(&app, e.message);
                    }
                });
            }
            "settings" => {
                let _ = present(app, "main");
                let _ = app.emit_to("main", "taskdock://open-settings", ());
            }
            "quit" => shutdown(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                }
            ) {
                let _ = present(tray.app_handle(), "main");
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}
