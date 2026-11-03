use crate::{desktop::DesktopRuntime, error::AppError, service::LocalService};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_updater::{Update, UpdaterExt};

pub const EVENT: &str = "taskdock://update-state";
const BUILD: &str = env!("TASKDOCK_BUILD_ID");

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    phase: String,
    current_build: String,
    available_build: Option<String>,
    notes: Option<String>,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    message: Option<String>,
}
impl Default for UpdateStatus {
    fn default() -> Self {
        Self {
            phase: "idle".into(),
            current_build: BUILD.into(),
            available_build: None,
            notes: None,
            downloaded_bytes: 0,
            total_bytes: None,
            message: None,
        }
    }
}
#[derive(Default)]
pub struct UpdateRuntime {
    status: Mutex<UpdateStatus>,
    pending: Mutex<Option<Update>>,
    bytes: Mutex<Option<Vec<u8>>>,
    busy: AtomicBool,
    cleanup_started: Arc<AtomicBool>,
}

fn error(message: &'static str) -> AppError {
    AppError {
        code: "UPDATE_FAILED",
        message,
    }
}
fn publish(app: &AppHandle, change: impl FnOnce(&mut UpdateStatus)) -> UpdateStatus {
    let rt = app.state::<UpdateRuntime>();
    let mut state = rt.status.lock().unwrap_or_else(|e| e.into_inner());
    change(&mut state);
    let result = state.clone();
    drop(state);
    let _ = app.emit(EVENT, &result);
    result
}
fn failure(app: &AppHandle, message: &'static str, cause: impl std::fmt::Display) -> AppError {
    log::warn!("updater: {cause}");
    publish(app, |s| {
        s.phase = "error".into();
        s.message = Some(message.into());
    });
    error(message)
}
struct Busy<'a>(&'a AtomicBool);
impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
fn acquire(rt: &UpdateRuntime) -> Result<Busy<'_>, AppError> {
    rt.busy
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .map_err(|_| error("正在检查、下载或安装更新，请稍候。"))?;
    Ok(Busy(&rt.busy))
}
fn require_main(window: &WebviewWindow) -> Result<(), AppError> {
    if window.label() != "main" {
        return Err(AppError::forbidden());
    }
    Ok(())
}
fn build_number(version: &str) -> Option<u64> {
    let suffix = version.strip_prefix("1.0.0+")?;
    if suffix.is_empty() || !suffix.bytes().all(|v| v.is_ascii_digit()) {
        return None;
    }
    suffix
        .parse::<u64>()
        .ok()
        .filter(|v| *v > 0 && *v <= 9_007_199_254_740_991)
}

#[tauri::command]
pub fn update_status(
    window: WebviewWindow,
    runtime: State<'_, UpdateRuntime>,
) -> Result<UpdateStatus, AppError> {
    require_main(&window)?;
    Ok(runtime.status.lock().map_err(AppError::internal)?.clone())
}

async fn check(app: &AppHandle) -> Result<UpdateStatus, AppError> {
    let rt = app.state::<UpdateRuntime>();
    let _busy = acquire(&rt)?;
    // A manual check must not discard a package already downloaded and verified.
    if rt.bytes.lock().map_err(AppError::internal)?.is_some() {
        return Ok(rt.status.lock().map_err(AppError::internal)?.clone());
    }
    publish(app, |s| {
        s.phase = "checking".into();
        s.message = None;
    });
    let shutdown_app = app.clone();
    let cleanup_started = rt.cleanup_started.clone();
    let exe = std::env::current_exe().map_err(AppError::internal)?;
    let install_directory = exe
        .parent()
        .ok_or_else(|| error("无法确定当前软件安装目录。"))?;
    // NSIS /D must be last and unquoted; spaces are parsed as part of its path.
    let installer_directory_argument = format!("/D={}", install_directory.display());
    let updater = app
        .updater_builder()
        .timeout(Duration::from_secs(20))
        .installer_arg(installer_directory_argument)
        .version_comparator(|_, release| {
            build_number(&release.version.to_string())
                .is_some_and(|v| v > BUILD.parse::<u64>().unwrap_or(u64::MAX))
        })
        .on_before_exit(move || {
            cleanup_started.store(true, Ordering::SeqCst);
            shutdown_app
                .state::<DesktopRuntime>()
                .quitting
                .store(true, Ordering::SeqCst);
            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            let _ = shutdown_app.global_shortcut().unregister_all();
            shutdown_app.cleanup_before_exit();
        })
        .build()
        .map_err(|e| failure(app, "更新配置不可用，请联系发布者。", e))?;
    // Checks use the same client as download; a finite request timeout also
    // makes an offline GitHub endpoint harmless to task operations.
    let result = updater
        .check()
        .await
        .map_err(|e| failure(app, "无法检查更新，请确认能访问GitHub后重试。", e))?;
    if let Some(mut update) = result {
        update.timeout = Some(Duration::from_secs(180));
        let path = update.download_url.path();
        if update.download_url.scheme() != "https"
            || update.download_url.host_str() != Some("github.com")
            || !path.starts_with("/ikunyu/TaskDock/releases/download/build-")
            || !path.ends_with("/TaskDock_1.0.0_x64-setup.exe")
        {
            return Err(failure(
                app,
                "更新下载地址不属于TaskDock官方发布仓库。",
                "release URL rejected",
            ));
        }
        let build = build_number(&update.version).ok_or_else(|| error("更新构建编号无效。"))?;
        let notes = update.body.as_ref().map(|s| s.chars().take(2000).collect());
        *rt.pending.lock().map_err(AppError::internal)? = Some(update);
        Ok(publish(app, |s| {
            s.phase = "available".into();
            s.available_build = Some(build.to_string());
            s.notes = notes;
            s.message = None;
        }))
    } else {
        *rt.pending.lock().map_err(AppError::internal)? = None;
        Ok(publish(app, |s| {
            s.phase = "up_to_date".into();
            s.available_build = None;
            s.notes = None;
            s.message = None;
        }))
    }
}

#[tauri::command]
pub async fn check_for_updates(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<UpdateStatus, AppError> {
    require_main(&window)?;
    check(&app).await
}
#[tauri::command]
pub async fn download_update(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<UpdateStatus, AppError> {
    require_main(&window)?;
    let rt = app.state::<UpdateRuntime>();
    let _busy = acquire(&rt)?;
    let update = rt
        .pending
        .lock()
        .map_err(AppError::internal)?
        .clone()
        .ok_or_else(|| error("请先检查是否有可用更新。"))?;
    publish(&app, |s| {
        s.phase = "downloading".into();
        s.downloaded_bytes = 0;
        s.total_bytes = None;
        s.message = None;
    });
    let mut downloaded = 0u64;
    let mut last_percent = u64::MAX;
    let bytes = update
        .download(
            |chunk, total| {
                downloaded = downloaded.saturating_add(chunk as u64);
                let percent = total
                    .filter(|v| *v > 0)
                    .map_or(downloaded / 262_144, |v| downloaded.saturating_mul(100) / v);
                if percent != last_percent {
                    last_percent = percent;
                    publish(&app, |s| {
                        s.downloaded_bytes = downloaded;
                        s.total_bytes = total;
                    });
                }
            },
            || {},
        )
        .await
        .map_err(|e| {
            failure(
                &app,
                "更新下载或签名验证失败，当前软件和任务数据未更改。",
                e,
            )
        })?;
    let length = bytes.len() as u64;
    *rt.bytes.lock().map_err(AppError::internal)? = Some(bytes);
    Ok(publish(&app, |s| {
        s.phase = "ready".into();
        s.downloaded_bytes = length;
        s.total_bytes = Some(length);
    }))
}

#[tauri::command]
pub async fn install_update(
    app: AppHandle,
    window: WebviewWindow,
    confirmed: bool,
) -> Result<(), AppError> {
    require_main(&window)?;
    if !confirmed {
        return Err(error("安装前请确认已保存所有窗口中的编辑内容。"));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let rt = app.state::<UpdateRuntime>();
        let _busy = acquire(&rt)?;
        let update = rt.pending.lock().map_err(AppError::internal)?.clone().ok_or_else(|| error("请先下载可用更新。"))?;
        let bytes = rt.bytes.lock().map_err(AppError::internal)?.take().ok_or_else(|| error("请先下载并验证更新包。"))?;
        update.verify(&bytes).map_err(|e| failure(&app, "更新包签名无效，安装已取消。", e))?;
        publish(&app, |s| { s.phase="installing".into(); s.message=None; });
        let service = app.state::<Arc<LocalService>>().inner().clone();
        if let Err(e) = crate::desktop::prepare_update(&app) {
            if app.state::<DesktopRuntime>().quitting.load(Ordering::SeqCst) {
                return Err(error("软件正在退出，已取消更新安装。"));
            }
            let _ = service.resume_after_update_error();
            return Err(failure(&app, "无法安全关闭数据库，安装已取消，请稍后重新下载更新。", e.message));
        }
        if let Err(e) = update.install(bytes) {
            if rt.cleanup_started.load(Ordering::SeqCst) {
                log::error!("installer launch failed after cleanup: {e}");
                #[cfg(windows)] {
                    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_OK, MB_ICONERROR};
                    let title:Vec<u16>="TaskDock\0".encode_utf16().collect();
                    let text:Vec<u16>="更新安装器启动失败，已保存的任务数据保留。请重新打开TaskDock后重试，或手动运行最新版安装包。\0".encode_utf16().collect();
                    unsafe { MessageBoxW(std::ptr::null_mut(),text.as_ptr(),title.as_ptr(),MB_OK|MB_ICONERROR); }
                }
                std::process::exit(1);
            } else {
                service.resume_after_update_error().map_err(AppError::service)?;
            }
            return Err(failure(&app, "安装更新失败，请重新打开TaskDock或手动运行最新版安装包。", e));
        }
        Ok(())
    }).await.map_err(AppError::internal)?
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(8));
        if app
            .state::<DesktopRuntime>()
            .quitting
            .load(Ordering::SeqCst)
        {
            return;
        }
        let _ = tauri::async_runtime::block_on(check(&app));
    });
}
