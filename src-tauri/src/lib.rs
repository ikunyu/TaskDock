mod commands;
mod data;
mod db;
mod desktop;
mod error;
mod schedule;
mod service;
mod storage;
mod task_commands;
mod tasks;
mod updates;

use std::sync::{atomic::Ordering, Arc};
use tauri::Manager;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

pub fn run() {
    let paths = match storage::Paths::current() {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("TaskDock executable path: {error}");
            storage::startup_error();
            return;
        }
    };
    let mut log_targets = vec![Target::new(TargetKind::Stdout)];
    if storage::writable_directory(&paths.logs).is_ok() {
        log_targets.push(Target::new(TargetKind::Folder {
            path: paths.logs.clone(),
            file_name: Some("taskdock".into()),
        }));
    }
    let result = tauri::Builder::default()
        // First plugin: reject the second instance before any LocalService exists.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = desktop::present(app, "main");
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, key, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        desktop::runtime_shortcut(app, key);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--background"]),
        ))
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets(log_targets)
                .level(log::LevelFilter::Info)
                .filter(|m| m.target().starts_with("taskdock"))
                .max_file_size(1_000_000)
                .rotation_strategy(RotationStrategy::KeepSome(3))
                .build(),
        )
        .setup(move |app| {
            let legacy_directory = app.path().app_local_data_dir()?;
            app.manage(Arc::new(service::LocalService::portable(
                paths.data.join("taskdock.sqlite3"),
                legacy_directory,
            )));
            app.manage(desktop::DesktopRuntime::default());
            app.manage(schedule::OpenTask::default());
            app.manage(updates::UpdateRuntime::default());
            for config in &app.config().app.windows {
                tauri::WebviewWindowBuilder::from_config(app, config)?
                    .data_directory(paths.webview.clone())
                    .build()?;
            }
            desktop::install_tray(app.handle())?;
            let background = std::env::args().any(|arg| arg == "--background");
            desktop::initialize(app.handle().clone(), background);
            schedule::start(app.handle().clone());
            data::start(app.handle().clone());
            updates::start(app.handle().clone());
            Ok(())
        })
        .on_window_event(desktop::window_event)
        .invoke_handler(tauri::generate_handler![
            data::data_status,
            data::save_backup_settings,
            data::backup_now,
            data::restore_backup,
            data::search_tasks,
            data::calendar_month,
            data::export_data,
            data::open_backup_directory,
            schedule::reminder_action,
            schedule::take_notification_task,
            commands::bootstrap,
            updates::update_status,
            updates::check_for_updates,
            updates::download_update,
            updates::install_update,
            commands::set_theme,
            commands::show_window,
            commands::hide_current_window,
            commands::report_frontend_error,
            task_commands::tasks_snapshot,
            task_commands::create_task,
            task_commands::update_task,
            task_commands::set_task_status,
            task_commands::purge_task,
            task_commands::save_category,
            task_commands::delete_category,
            desktop::desktop_status,
            desktop::save_desktop_settings,
            desktop::reset_window_positions,
            desktop::exit_app,
            desktop::widget_window_action,
            desktop::lock_widget,
            desktop::set_widget_layout
        ])
        .build(tauri::generate_context!());
    match result {
        Ok(app) => app.run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if let Some(rt) = app.try_state::<desktop::DesktopRuntime>() {
                    if !rt.quitting.load(Ordering::SeqCst) {
                        api.prevent_exit();
                        desktop::shutdown(app);
                    }
                }
            }
        }),
        Err(error) => {
            eprintln!("TaskDock startup failed. Check the application log.");
            log::error!("TaskDock startup: {error}");
            storage::startup_error();
            std::process::exit(1);
        }
    }
}
