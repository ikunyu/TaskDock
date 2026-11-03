use super::{
    backup,
    export::{self, ExportFormat},
    model::*,
};
use crate::{
    error::{AppError, ServiceError},
    service::LocalService,
};
use std::{
    fs::{self, OpenOptions},
    sync::Arc,
};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

fn authorize(w: &WebviewWindow) -> Result<(), AppError> {
    if w.label() == "main" {
        Ok(())
    } else {
        Err(AppError::forbidden())
    }
}
fn data_error(e: ServiceError) -> AppError {
    match e {
        ServiceError::Validation(_) | ServiceError::Conflict | ServiceError::NotFound => {
            AppError::service(e)
        }
        e => {
            log::error!("data operation failed: {e}");
            AppError {
                code: "DATA_OPERATION_FAILED",
                message: "本地数据操作失败，请重试；如持续出现，请查看应用日志获取具体原因。",
            }
        }
    }
}
async fn run<T: Send + 'static>(
    s: &Arc<LocalService>,
    f: impl FnOnce(&LocalService) -> Result<T, ServiceError> + Send + 'static,
) -> Result<T, AppError> {
    let s = s.clone();
    tauri::async_runtime::spawn_blocking(move || f(&s))
        .await
        .map_err(AppError::internal)?
        .map_err(data_error)
}
#[tauri::command]
pub async fn data_status(
    window: WebviewWindow,
    service: State<'_, Arc<LocalService>>,
) -> Result<DataStatus, AppError> {
    authorize(&window)?;
    run(service.inner(), LocalService::data_status).await
}
#[tauri::command]
pub async fn save_backup_settings(
    app: AppHandle,
    window: WebviewWindow,
    service: State<'_, Arc<LocalService>>,
    settings: BackupSettings,
) -> Result<DataStatus, AppError> {
    authorize(&window)?;
    let result = run(service.inner(), move |s| {
        s.with_database(|db| db.save_backup_settings(settings))?;
        s.data_status()
    })
    .await?;
    let settings = run(service.inner(), LocalService::settings).await?;
    let _ = app.emit(crate::commands::SETTINGS_EVENT, settings);
    let _ = app.emit("taskdock://data-changed", ());
    Ok(result)
}
#[tauri::command]
pub async fn backup_now(
    app: AppHandle,
    window: WebviewWindow,
    service: State<'_, Arc<LocalService>>,
) -> Result<BackupEntry, AppError> {
    authorize(&window)?;
    let r = run(service.inner(), |s| s.create_backup("manual")).await;
    let _ = app.emit("taskdock://data-changed", ());
    r
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub safety_id: String,
    pub warnings: Vec<String>,
}
#[tauri::command]
pub async fn restore_backup(
    app: AppHandle,
    window: WebviewWindow,
    service: State<'_, Arc<LocalService>>,
    id: String,
    confirmed: bool,
) -> Result<RestoreResult, AppError> {
    authorize(&window)?;
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let rt = app.state::<crate::desktop::DesktopRuntime>();
        let _gate = rt.gate.lock().map_err(AppError::internal)?;
        rt.restoring
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let result = service.restore_backup(&id, confirmed).map_err(data_error);
        rt.restoring
            .store(false, std::sync::atomic::Ordering::SeqCst);
        let safety_id = result?;
        let warnings = crate::desktop::reload_after_restore(&app);
        let _=app.emit("taskdock://database-restored",serde_json::json!({"epoch":service.epoch.load(std::sync::atomic::Ordering::SeqCst),"safetyId":safety_id,"warnings":warnings}));
        let _ = app.emit("taskdock://data-changed", ());
        Ok(RestoreResult {
            safety_id,
            warnings,
        })
    })
    .await
    .map_err(AppError::internal)?
}
#[tauri::command]
pub async fn search_tasks(
    window: WebviewWindow,
    service: State<'_, Arc<LocalService>>,
    filter: SearchFilter,
) -> Result<SearchPage, AppError> {
    authorize(&window)?;
    run(service.inner(), move |s| {
        s.with_database(|db| db.search(filter))
    })
    .await
}
#[tauri::command]
pub async fn calendar_month(
    window: WebviewWindow,
    service: State<'_, Arc<LocalService>>,
    month: String,
    category_id: Option<String>,
) -> Result<Vec<CalendarDay>, AppError> {
    authorize(&window)?;
    run(service.inner(), move |s| {
        s.with_database(|db| db.calendar(&month, category_id))
    })
    .await
}
#[tauri::command]
pub async fn export_data(
    app: AppHandle,
    window: WebviewWindow,
    service: State<'_, Arc<LocalService>>,
    format: ExportFormat,
    filter: Option<SearchFilter>,
) -> Result<Option<String>, AppError> {
    authorize(&window)?;
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let ext = export::extension(format);
        let Some(chosen) = app
            .dialog()
            .file()
            .set_title("导出 TaskDock 数据")
            .add_filter("TaskDock导出", &[ext])
            .set_file_name(format!(
                "taskdock-export-{}.{}",
                chrono::Local::now().format("%Y%m%d-%H%M%S"),
                ext
            ))
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let mut path = chosen.into_path().map_err(AppError::internal)?;
        if path.extension().is_none() {
            path.set_extension(ext);
        }
        if path.extension().and_then(|e| e.to_str()) != Some(ext) {
            return Err(AppError::service(ServiceError::Validation(
                "导出扩展名与所选格式不一致。",
            )));
        }
        let dir = path
            .parent()
            .ok_or_else(|| AppError::service(ServiceError::Validation("导出目录无效。")))?;
        let protected = service
            .path
            .parent()
            .ok_or_else(|| AppError::internal("database parent"))?
            .canonicalize()
            .map_err(AppError::internal)?;
        if dir
            .canonicalize()
            .map_err(AppError::internal)?
            .starts_with(&protected)
            || path
                .symlink_metadata()
                .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err(AppError::service(ServiceError::Validation(
                "不能导出到应用数据库目录或符号链接，请选择其他位置。",
            )));
        }
        if path.exists()
            && !app
                .dialog()
                .message("目标文件已存在，是否替换此导出文件？")
                .title("确认替换导出文件")
                .buttons(MessageDialogButtons::OkCancel)
                .blocking_show()
        {
            return Ok(None);
        }
        let temp = dir.join(format!(
            ".taskdock-export-{}.partial",
            crate::tasks::validation::id()
        ));
        let result = (|| {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temp)?;
            service.with_database(|_db| {
                // One scoped read transaction under the service lifecycle lease;
                // no second task service and no frontend filesystem access.
                let mut source = backup::readonly(&service.path)?;
                export::write_connection(&mut source, &mut file, format, filter)
            })?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temp, &path)?;
            Ok::<_, ServiceError>(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
        }
        result.map_err(data_error)?;
        Ok(Some(
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
        ))
    })
    .await
    .map_err(AppError::internal)?
}
#[tauri::command]
pub async fn open_backup_directory(
    window: WebviewWindow,
    service: State<'_, Arc<LocalService>>,
) -> Result<(), AppError> {
    authorize(&window)?;
    run(service.inner(), |s| {
        let dir = backup::directory(&s.path)?;
        fs::create_dir_all(&dir)?;
        #[cfg(windows)]
        {
            std::process::Command::new("explorer.exe")
                .arg(dir)
                .spawn()?;
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = dir;
            Err(ServiceError::Validation("打开备份目录仅支持Windows。"))
        }
    })
    .await
}
