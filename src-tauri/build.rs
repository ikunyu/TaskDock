fn main() {
    println!("cargo:rerun-if-changed=../release-config.json");
    let release: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string("../release-config.json").expect("release-config.json missing"),
    )
    .expect("release configuration invalid");
    let build = release["build"]
        .as_u64()
        .filter(|v| *v > 0 && *v <= 9_007_199_254_740_991)
        .expect("invalid update build number");
    assert_eq!(release["repository"].as_str(), Some("ikunyu/TaskDock"));
    assert_eq!(
        release["appVersion"].as_str(),
        Some(env!("CARGO_PKG_VERSION"))
    );
    println!("cargo:rustc-env=TASKDOCK_BUILD_ID={build}");
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "data_status",
            "save_backup_settings",
            "backup_now",
            "restore_backup",
            "search_tasks",
            "calendar_month",
            "export_data",
            "open_backup_directory",
            "reminder_action",
            "take_notification_task",
            "desktop_status",
            "save_desktop_settings",
            "reset_window_positions",
            "exit_app",
            "widget_window_action",
            "lock_widget",
            "unlock_widget",
            "set_widget_lock_bounds",
            "widget_lock_control_ready",
            "set_widget_layout",
            "bootstrap",
            "update_status",
            "check_for_updates",
            "download_update",
            "install_update",
            "set_theme",
            "show_window",
            "hide_current_window",
            "report_frontend_error",
            "tasks_snapshot",
            "create_task",
            "update_task",
            "set_task_status",
            "purge_task",
            "save_category",
            "delete_category",
        ]),
    ))
    .expect("Tauri build configuration is invalid");
}
