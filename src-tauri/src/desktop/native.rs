/// Called from desktop workers, never the UI thread. Keep Tao's cursor/focus
/// flags and Windows layered alpha in order on the same UI-thread turn. Updating
/// ignore-cursor flags can remove WS_EX_LAYERED, so alpha must be applied last.
pub fn widget_style(window: &tauri::WebviewWindow, percent: u8, locked: bool) -> tauri::Result<()> {
    on_window_thread(window, move |target| {
        target.set_ignore_cursor_events(locked)?;
        target.set_focusable(!locked)?;
        opacity(target, percent)
    })
}
/// Re-assert the configured topmost state only when the live window disagrees.
/// Rewriting GWL_EXSTYLE for the layered/cursor flags can drop WS_EX_TOPMOST, and
/// the startup apply can run before the widget window exists, so the state is
/// checked here instead of assuming the earlier call still holds.
pub fn ensure_topmost(window: &tauri::WebviewWindow, topmost: bool) -> tauri::Result<()> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetWindowLongW, GWL_EXSTYLE, WS_EX_TOPMOST,
        };
        let hwnd = window.hwnd()?.0 as windows_sys::Win32::Foundation::HWND;
        // SAFETY: the handle belongs to this process's own widget window and is
        // only read, never modified here.
        let style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) };
        let is_topmost = (style as u32 & WS_EX_TOPMOST) != 0;
        if is_topmost == topmost {
            return Ok(());
        }
    }
    window.set_always_on_top(topmost)
}
pub fn lock_control_style(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    on_window_thread(window, move |target| {
        target.set_focusable(false)?;
        // Input-only hotspot. Alpha=0 makes Win32 pass mouse input through;
        // the minimum nonzero alpha keeps the whole button clickable. The
        // visible icon is drawn once by the widget at its configured opacity.
        layered_alpha(target, 1)
    })
}
fn on_window_thread(
    window: &tauri::WebviewWindow,
    operation: impl FnOnce(&tauri::WebviewWindow) -> tauri::Result<()> + Send + 'static,
) -> tauri::Result<()> {
    let target = window.clone();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    window.run_on_main_thread(move || {
        let result = operation(&target);
        let _ = send.send(result.map_err(|e: tauri::Error| e.to_string()));
    })?;
    receive
        .recv()
        .map_err(std::io::Error::other)?
        .map_err(|e| std::io::Error::other(e).into())
}

pub fn opacity(window: &tauri::WebviewWindow, percent: u8) -> tauri::Result<()> {
    layered_alpha(window, ((percent as u32 * 255 + 50) / 100) as u8)
}
#[cfg(windows)]
fn layered_alpha(window: &tauri::WebviewWindow, alpha: u8) -> tauri::Result<()> {
    use windows_sys::Win32::{
        Foundation::{GetLastError, SetLastError},
        UI::WindowsAndMessaging::{
            GetWindowLongW, SetLayeredWindowAttributes, SetWindowLongW, GWL_EXSTYLE, LWA_ALPHA,
            WS_EX_LAYERED,
        },
    };
    let hwnd = window.hwnd()?.0 as windows_sys::Win32::Foundation::HWND;
    // SAFETY: this is the live native HWND obtained from our widget. Only its
    // layered alpha/style are changed, never another process or arbitrary handle.
    unsafe {
        SetLastError(0);
        let style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        if style == 0 && GetLastError() != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        SetLastError(0);
        if SetWindowLongW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED as i32) == 0
            && GetLastError() != 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        if SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA) == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    Ok(())
}
#[cfg(not(windows))]
fn layered_alpha(_window: &tauri::WebviewWindow, _alpha: u8) -> tauri::Result<()> {
    Ok(())
}
