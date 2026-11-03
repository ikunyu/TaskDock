#[cfg(windows)]
pub fn opacity(window: &tauri::WebviewWindow, percent: u8) -> tauri::Result<()> {
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
        if SetLayeredWindowAttributes(
            hwnd,
            0,
            ((percent as u32 * 255 + 50) / 100) as u8,
            LWA_ALPHA,
        ) == 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    Ok(())
}
#[cfg(not(windows))]
pub fn opacity(_window: &tauri::WebviewWindow, _percent: u8) -> tauri::Result<()> {
    Ok(())
}
