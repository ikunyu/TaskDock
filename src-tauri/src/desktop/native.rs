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
#[cfg(windows)]
static FULLSCREEN_EVENTS: std::sync::OnceLock<std::sync::Mutex<std::sync::mpsc::Sender<()>>> =
    std::sync::OnceLock::new();
/// Delivered on the hook thread and only forwards the fact that some window
/// changed; reading the actual state happens once per burst on that thread.
#[cfg(windows)]
unsafe extern "system" fn fullscreen_event(
    _hook: windows_sys::Win32::UI::Accessibility::HWINEVENTHOOK,
    _event: u32,
    _window: windows_sys::Win32::Foundation::HWND,
    object: i32,
    child: i32,
    _thread: u32,
    _time: u32,
) {
    if object != 0 || child != 0 {
        return;
    }
    if let Some(lock) = FULLSCREEN_EVENTS.get() {
        if let Ok(sender) = lock.lock() {
            let _ = sender.send(());
        }
    }
}
/// Subscribe to foreground switches and window resize/move events instead of
/// polling. The returned receiver fires on every interesting window change, so
/// the caller can read the full-screen state only when something happened. The
/// hook thread blocks in its message loop and costs nothing while idle.
#[cfg(windows)]
pub fn fullscreen_events() -> std::sync::mpsc::Receiver<()> {
    let (sender, receiver) = std::sync::mpsc::channel();
    let _ = FULLSCREEN_EVENTS.set(std::sync::Mutex::new(sender));
    let _ = std::thread::Builder::new()
        .name("taskdock-fullscreen".into())
        .spawn(|| unsafe {
            use windows_sys::Win32::UI::{
                Accessibility::SetWinEventHook,
                WindowsAndMessaging::{
                    DispatchMessageW, GetMessageW, EVENT_OBJECT_LOCATIONCHANGE,
                    EVENT_SYSTEM_FOREGROUND, MSG, WINEVENT_OUTOFCONTEXT,
                },
            };
            let foreground = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                std::ptr::null_mut(),
                Some(fullscreen_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            let moved = SetWinEventHook(
                EVENT_OBJECT_LOCATIONCHANGE,
                EVENT_OBJECT_LOCATIONCHANGE,
                std::ptr::null_mut(),
                Some(fullscreen_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            if foreground.is_null() && moved.is_null() {
                return;
            }
            let mut message: MSG = std::mem::zeroed();
            while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                DispatchMessageW(&message);
            }
        });
    receiver
}
#[cfg(not(windows))]
pub fn fullscreen_events() -> std::sync::mpsc::Receiver<()> {
    std::sync::mpsc::channel().1
}
/// True when a full-screen application owns the foreground: either the shell
/// reports a full-screen/presentation state, or the foreground window of another
/// process covers its monitor edge to edge. Shell windows are ignored so that
/// clicking the desktop does not count as full screen.
#[cfg(windows)]
pub fn fullscreen_foreground() -> bool {
    use windows_sys::Win32::{
        Foundation::RECT,
        Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        },
        UI::{
            Shell::{
                SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE,
                QUNS_RUNNING_D3D_FULL_SCREEN,
            },
            WindowsAndMessaging::{
                GetClassNameW, GetForegroundWindow, GetWindowLongW, GetWindowRect,
                GetWindowThreadProcessId, GWL_STYLE, WS_CAPTION,
            },
        },
    };
    unsafe {
        let mut state: i32 = 0;
        if SHQueryUserNotificationState(&mut state) == 0
            && matches!(
                state,
                QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE
            )
        {
            return true;
        }
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == std::process::id() {
            return false;
        }
        // The desktop and the taskbar cover their monitor too.
        let mut class = [0u16; 64];
        let length = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32);
        let class =
            String::from_utf16_lossy(&class[..length.clamp(0, class.len() as i32) as usize]);
        if matches!(
            class.as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
        ) {
            return false;
        }
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return false;
        }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if monitor.is_null() {
            return false;
        }
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return false;
        }
        if !(rect.left <= info.rcMonitor.left
            && rect.top <= info.rcMonitor.top
            && rect.right >= info.rcMonitor.right
            && rect.bottom >= info.rcMonitor.bottom)
        {
            return false;
        }
        // A maximized window keeps its caption and, with an auto-hide taskbar,
        // covers the monitor exactly like a full-screen app does.
        let style = GetWindowLongW(hwnd, GWL_STYLE);
        (style as u32 & WS_CAPTION) == 0
    }
}
#[cfg(not(windows))]
pub fn fullscreen_foreground() -> bool {
    false
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
