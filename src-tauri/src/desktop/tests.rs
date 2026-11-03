use super::{
    geometry::{fit, WorkArea},
    settings::{normalize_shortcut, DesktopSettings, Placement},
};
use crate::{db::Database, error::ServiceError};
fn area() -> WorkArea {
    WorkArea {
        x: 0,
        y: 0,
        width: 1920,
        height: 1040,
        scale: 1.0,
        primary: true,
    }
}
fn placement() -> Placement {
    Placement {
        label: "widget".into(),
        x: 150,
        y: 120,
        width: 320.0,
        height: 400.0,
        scale: 1.0,
    }
}
#[test]
fn geometry_handles_negative_monitor_and_changed_dpi() {
    let secondary = WorkArea {
        x: -2560,
        y: 0,
        width: 2560,
        height: 1400,
        scale: 2.0,
        primary: false,
    };
    let mut p = placement();
    p.x = -2400;
    p.y = 100;
    let result = fit("widget", Some(&p), &[area(), secondary]).unwrap();
    assert_eq!(result.x, -2400);
    assert_eq!(result.width, 320.0);
    assert_eq!(result.scale, 2.0);
}
#[test]
fn disconnected_monitor_and_small_workarea_use_safe_fallback() {
    let mut p = placement();
    p.x = -2400;
    p.y = -900;
    let result = fit("widget", Some(&p), &[area()]).unwrap();
    assert!(result.x >= 0 && result.y >= 0);
    assert!(result.x + 320 <= 1920);
    let small = WorkArea {
        width: 800,
        height: 600,
        scale: 1.5,
        ..area()
    };
    let result = fit(
        "widget",
        Some(&Placement {
            width: 600.0,
            height: 800.0,
            ..p
        }),
        &[small],
    )
    .unwrap();
    assert!(result.height * 1.5 + 60.0 <= 600.0);
    assert!(result.width * 1.5 <= 800.0);
}
#[test]
fn resolution_change_clamps_bottom_right_and_rejects_nan() {
    let mut p = placement();
    p.x = 1800;
    p.y = 950;
    let result = fit("widget", Some(&p), &[area()]).unwrap();
    assert!(result.x <= 1584);
    assert!(result.y <= 600);
    p.width = f64::NAN;
    let result = fit("widget", Some(&p), &[area()]).unwrap();
    assert_eq!(result.width, 180.0);
    p.x = i32::MIN;
    assert!(!p.valid());
    assert!(fit("widget", None, &[]).is_none());
}
#[test]
fn shortcut_validation_normalizes_and_rejects_unsafe_or_incomplete_values() {
    assert_eq!(normalize_shortcut("alt+control+t").unwrap(), "Ctrl+Alt+T");
    for key in [
        "T",
        "Shift+T",
        "Ctrl+Ctrl+T",
        "Win+T",
        "Ctrl+Alt+",
        "Ctrl+F13",
        "Ctrl+Escape",
    ] {
        assert!(normalize_shortcut(key).is_err(), "{key}");
    }
    assert_eq!(normalize_shortcut("Ctrl+F12").unwrap(), "Ctrl+F12");
}
#[test]
fn desktop_persistence_revision_conflicts_and_validation_preserve_tasks() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("desktop.sqlite3");
    let db = Database::open(&path).unwrap();
    let task = db
        .create_task(crate::tasks::TaskDraft {
            title: "保留任务".into(),
            ..Default::default()
        })
        .unwrap();
    let initial = db.settings().unwrap();
    assert!(!initial.desktop.autostart);
    assert_eq!(initial.desktop.widget_opacity, 100);
    let mut changed = initial.desktop.clone();
    changed.widget_opacity = 65;
    changed.widget_visible = true;
    let result = db.save_desktop(changed.clone(), initial.revision).unwrap();
    assert_eq!(result.revision, initial.revision + 1);
    assert!(matches!(
        db.save_desktop(changed.clone(), initial.revision),
        Err(ServiceError::Conflict)
    ));
    changed.widget_opacity = 19;
    assert!(db.save_desktop(changed, result.revision).is_err());
    assert_eq!(db.settings().unwrap().revision, result.revision);
    assert_eq!(db.task_snapshot().unwrap().revision, task.snapshot.revision);
    db.save_placements(&[placement()]).unwrap();
    drop(db);
    let db = Database::open(&path).unwrap();
    assert_eq!(db.desktop_settings().unwrap().widget_opacity, 65);
    assert_eq!(db.placements().unwrap(), vec![placement()]);
    assert_eq!(db.task_snapshot().unwrap().tasks.len(), 1);
}
#[test]
fn invalid_preferences_and_positions_cannot_enter_database() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(&dir.path().join("desktop.sqlite3")).unwrap();
    let c = db.connection.lock().unwrap();
    assert!(c
        .execute("UPDATE desktop_settings SET widget_opacity=19", [])
        .is_err());
    assert!(c
        .execute("UPDATE desktop_settings SET widget_locked=2", [])
        .is_err());
    assert!(c
        .execute("UPDATE desktop_settings SET close_behavior='invalid'", [])
        .is_err());
    drop(c);
    assert!(db
        .save_placements(&[Placement {
            scale: 0.0,
            ..placement()
        }])
        .is_err());
    let mut prefs = DesktopSettings {
        shortcut: "Ctrl+Alt+T".into(),
        ..Default::default()
    };
    prefs.close_behavior = "other".into();
    assert!(prefs.validate().is_err());
}
#[test]
fn database_shutdown_prevents_reopening_service() {
    let dir = tempfile::tempdir().unwrap();
    let service = crate::service::LocalService::new(dir.path().join("closed.sqlite3"));
    service.settings().unwrap();
    service.shutdown().unwrap();
    assert!(service.settings().is_err());
}

#[test]
fn candidate_shortcut_conflict_never_unregisters_old_key() {
    use std::{cell::RefCell, collections::HashSet};
    let registered = RefCell::new(HashSet::from([
        "Ctrl+Alt+T".to_string(),
        "Ctrl+Alt+Y".into(),
    ]));
    let result = super::shortcut::ShortcutLease::prepare(
        Some("Ctrl+Alt+T"),
        "Ctrl+Alt+Y",
        |key| {
            if registered.borrow().contains(key) {
                Err(crate::error::AppError {
                    code: "SHORTCUT_CONFLICT",
                    message: "conflict",
                })
            } else {
                registered.borrow_mut().insert(key.into());
                Ok(())
            }
        },
        |key| {
            registered.borrow_mut().remove(key);
            Ok(())
        },
    );
    assert!(result.is_err());
    assert!(registered.borrow().contains("Ctrl+Alt+T"));
}
#[test]
fn failed_persistence_releases_candidate_but_success_keeps_it() {
    use std::{cell::RefCell, collections::HashSet};
    let registered = RefCell::new(HashSet::from(["Ctrl+Alt+T".to_string()]));
    let prepare = || {
        super::shortcut::ShortcutLease::prepare(
            Some("Ctrl+Alt+T"),
            "Ctrl+Alt+U",
            |key| {
                registered.borrow_mut().insert(key.into());
                Ok(())
            },
            |key| {
                registered.borrow_mut().remove(key);
                Ok(())
            },
        )
        .unwrap()
    };
    {
        let _uncommitted = prepare();
        assert!(registered.borrow().contains("Ctrl+Alt+T"));
        assert!(registered.borrow().contains("Ctrl+Alt+U"));
    }
    assert_eq!(registered.borrow().len(), 1);
    let committed = prepare();
    committed.commit();
    assert_eq!(registered.borrow().len(), 2); // old key is removed only after commit by runtime
}
#[test]
fn invalid_saved_preferences_fall_back_without_touching_task_data() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(&dir.path().join("invalid.sqlite3")).unwrap();
    db.connection.lock().unwrap().execute_batch("PRAGMA ignore_check_constraints=ON; UPDATE desktop_settings SET widget_opacity=999,close_behavior='invalid';").unwrap();
    assert_eq!(db.settings().unwrap().desktop, DesktopSettings::default());
}

#[test]
fn unchanged_unavailable_shortcut_does_not_register_while_saving_other_settings() {
    use std::cell::Cell;
    let attempts = Cell::new(0);
    let lease = super::shortcut::ShortcutLease::for_settings(
        "Ctrl+Alt+T",
        None,
        "Ctrl+Alt+T",
        |_| {
            attempts.set(attempts.get() + 1);
            Err(crate::error::AppError {
                code: "SHORTCUT_CONFLICT",
                message: "occupied",
            })
        },
        |_| panic!("unchanged shortcut must not be unregistered"),
    )
    .unwrap();
    assert!(lease.is_none());
    assert_eq!(attempts.get(), 0);
    let changed = super::shortcut::ShortcutLease::for_settings(
        "Ctrl+Alt+T",
        None,
        "Ctrl+Alt+Y",
        |_| {
            attempts.set(attempts.get() + 1);
            Err(crate::error::AppError {
                code: "SHORTCUT_CONFLICT",
                message: "occupied",
            })
        },
        |_| panic!("failed registration must not unregister someone else's key"),
    );
    assert!(changed.is_err());
    assert_eq!(attempts.get(), 1);
}

#[test]
fn visibility_changes_only_its_field_and_no_op_preserves_revision() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("visibility.sqlite3");
    let db = Database::open(&path).unwrap();
    let before = db.settings().unwrap();
    let custom = DesktopSettings {
        shortcut: "Ctrl+Alt+Y".into(),
        widget_opacity: 73,
        widget_topmost: true,
        autostart: true,
        ..before.desktop
    };
    let saved = db.save_desktop(custom.clone(), before.revision).unwrap();
    let task_revision = db.task_snapshot().unwrap().revision;
    let visible = db.set_widget_visibility(true).unwrap();
    let mut expected = custom;
    expected.widget_visible = true;
    assert_eq!(visible.desktop, expected);
    assert_eq!(visible.revision, saved.revision + 1);
    assert_eq!(
        db.set_widget_visibility(true).unwrap().revision,
        visible.revision
    );
    assert_eq!(db.task_snapshot().unwrap().revision, task_revision);
    drop(db);
    assert_eq!(
        Database::open(&path).unwrap().settings().unwrap().desktop,
        expected
    );
}

#[test]
fn lock_and_twenty_percent_opacity_persist_without_touching_tasks() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("locked.sqlite3");
    let db = Database::open(&path).unwrap();
    let before = db.settings().unwrap();
    let mut prefs = before.desktop;
    prefs.widget_opacity = 20;
    let saved = db.save_desktop(prefs, before.revision).unwrap();
    let task_revision = db.task_snapshot().unwrap().revision;
    let locked = db.set_widget_locked(true).unwrap();
    assert!(locked.desktop.widget_locked);
    assert_eq!(locked.desktop.widget_opacity, 20);
    assert_eq!(locked.revision, saved.revision + 1);
    assert_eq!(
        db.set_widget_locked(true).unwrap().revision,
        locked.revision
    );
    assert_eq!(db.task_snapshot().unwrap().revision, task_revision);
    drop(db);
    let db = Database::open(&path).unwrap();
    let reopened = db.settings().unwrap();
    assert_eq!(reopened.desktop, locked.desktop);
    assert_eq!(reopened.revision, locked.revision);
    let unlocked = db.set_widget_locked(false).unwrap();
    assert!(!unlocked.desktop.widget_locked);
    assert_eq!(unlocked.desktop.widget_opacity, 20);
}
