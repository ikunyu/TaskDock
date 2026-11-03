use crate::{desktop::DesktopRuntime, service::LocalService};
use std::{
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

pub fn start(app: AppHandle) {
    // Registered once during setup, never per WebView or per scheduling tick.
    let click_app = app.clone();
    if let Err(e) = app.notification().on_action(move |action| {
        if action.action_id() != "tap" {
            return;
        }
        let json = serde_json::to_value(action).unwrap_or_default();
        let id = json
            .pointer("/notification/extra/taskId")
            .and_then(|v| v.as_str())
            .filter(|id| uuid::Uuid::parse_str(id).is_ok())
            .map(str::to_owned);
        if let Ok(mut pending) = click_app.state::<super::OpenTask>().0.lock() {
            *pending = id;
        }
        let _ = crate::desktop::present(&click_app, "main");
        let _ = click_app.emit_to("main", "taskdock://notification-open", ());
    }) {
        log::warn!("notification click handler unavailable: {e}");
    }
    std::thread::spawn(move || {
        let service = app.state::<Arc<LocalService>>().inner().clone();
        let mut last_send = Instant::now()
            .checked_sub(Duration::from_secs(60))
            .unwrap_or_else(Instant::now);
        while !app
            .state::<DesktopRuntime>()
            .quitting
            .load(Ordering::SeqCst)
        {
            // One merged toast per 30 seconds, bounded catch-up after sleep/restart.
            if last_send.elapsed() >= Duration::from_secs(30) {
                let epoch = service.epoch.load(Ordering::SeqCst);
                let result = service.with_database(|db| db.schedule_tick());
                match result {
                    Ok(Some((result, batch))) => {
                        if epoch != service.epoch.load(Ordering::SeqCst) {
                            continue;
                        }
                        crate::task_commands::publish(&app, result);
                        if app
                            .state::<DesktopRuntime>()
                            .quitting
                            .load(Ordering::SeqCst)
                        {
                            break;
                        }
                        if let Err(e)=service.with_database(|db|{if epoch!=service.epoch.load(Ordering::SeqCst){return Ok(());}db.dispatch_reminders(&batch, |batch| {
                            if !batch.is_empty() {
                            let title = if batch.len() == 1 {
                                batch[0].title.chars().take(120).collect::<String>()
                            } else {
                                format!("TaskDock · {} 条到期提醒", batch.len())
                            };
                            let body = if batch.len() == 1 {
                                format!(
                                    "任务提醒 · {}\n可在 TaskDock 提醒中心稍后提醒。",
                                    batch[0].scheduled_at
                                )
                            } else {
                                format!(
                                    "{}\n打开 TaskDock 提醒中心查看全部。",
                                    batch
                                        .iter()
                                        .take(3)
                                        .map(|r| r.title.chars().take(60).collect::<String>())
                                        .collect::<Vec<_>>()
                                        .join("、")
                                )
                            };
                            // Persisted inbox is ALWAYS available: desktop permission_state
                            // reports Granted even when Windows suppresses notifications.
                            let send = app
                                .notification()
                                .builder()
                                .title(title)
                                .body(body)
                                .extra(
                                    "taskId",
                                    if batch.len() == 1 {
                                        Some(batch[0].task_id.clone())
                                    } else {
                                        None
                                    },
                                )
                                .show();
                            if let Err(e) = send {
                                log::warn!("notification preparation failed; persistent inbox retained: {e}");
                            }
                            last_send = Instant::now();
                        }
                        })}) { log::warn!("notification dispatch deferred; inbox retained: {e}"); }
                    }
                    Ok(None) => {}
                    Err(e) => log::warn!("scheduler tick deferred: {e}"),
                }
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}
