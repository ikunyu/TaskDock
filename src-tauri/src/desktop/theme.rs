use crate::db::Theme;
use tauri::{AppHandle, Manager};

/// Keep the native frame in step with the persisted CSS theme. None restores
/// OS theme tracking instead of freezing the current system appearance.
pub fn apply(app: &AppHandle, preference: Theme) {
    let theme = match preference {
        Theme::System => None,
        Theme::Light => Some(tauri::Theme::Light),
        Theme::Dark => Some(tauri::Theme::Dark),
    };
    for (label, window) in app.webview_windows() {
        if let Err(error) = window.set_theme(theme) {
            log::warn!("native theme update failed for {label}: {error}");
            super::runtime::warn(app, "窗口标题栏主题同步失败，请重新切换主题或重启应用。");
        }
    }
}
