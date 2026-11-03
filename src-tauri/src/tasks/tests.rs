use super::*;
use crate::{db::Database, error::ServiceError};

fn database() -> (tempfile::TempDir, Database) {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(&directory.path().join("tasks.sqlite3")).unwrap();
    (directory, database)
}

#[test]
fn creation_date_policy_accepts_today_future_and_wishes_without_blocking_history() {
    use chrono::{Local, NaiveDate, TimeZone};
    let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
    let mut value = draft("new date policy");
    assert!(validation::creation_dates(&value, today).is_ok());
    value.planned_date = Some("2026-10-29".into());
    assert!(validation::creation_dates(&value, today).is_err());
    assert!(validation::draft(value.clone()).is_ok());
    value.planned_date = Some("2026-10-30".into());
    assert!(validation::creation_dates(&value, today).is_ok());
    value.planned_date = Some("2026-10-31".into());
    value.due_date = Some("2026-10-29".into());
    assert!(validation::creation_dates(&value, today).is_err());
    value.due_date = None;
    value.due_at = Some(
        Local
            .with_ymd_and_hms(2026, 10, 29, 12, 0, 0)
            .single()
            .unwrap()
            .to_rfc3339(),
    );
    assert!(validation::creation_dates(&value, today).is_err());
    value.due_at = Some(
        Local
            .with_ymd_and_hms(2026, 10, 30, 12, 0, 0)
            .single()
            .unwrap()
            .to_rfc3339(),
    );
    assert!(validation::creation_dates(&value, today).is_ok());
}

#[test]
fn permanent_deletion_accepts_active_tasks_and_preserves_legacy_deleted_records() {
    let (_directory, database) = database();
    let mut value = draft("permanently remove");
    value.subtasks.push(SubtaskDraft {
        id: None,
        title: "child".into(),
        completed: false,
    });
    let active = find(&database.create_task(value).unwrap());
    let legacy = find(&database.create_task(draft("legacy removed")).unwrap());
    let legacy = find(
        &database
            .set_task_deleted(&legacy.id, legacy.version, true)
            .unwrap(),
    );
    let revision = database.task_snapshot().unwrap().revision;
    assert!(database
        .purge_task(&active.id, active.version, false)
        .is_err());
    assert!(matches!(
        database.purge_task(&active.id, active.version + 1, true),
        Err(ServiceError::Conflict)
    ));
    assert_eq!(database.task_snapshot().unwrap().revision, revision);
    let result = database
        .purge_task(&active.id, active.version, true)
        .unwrap();
    assert_eq!(result.snapshot.revision, revision + 1);
    assert_eq!(result.snapshot.tasks.len(), 1);
    assert_eq!(result.snapshot.tasks[0].id, legacy.id);
    assert!(result.snapshot.tasks[0].deleted_at.is_some());
    let connection = database.connection.lock().unwrap();
    for table in ["subtasks", "reminders", "task_schedule", "task_history"] {
        let count: i64 = connection
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE task_id=?1"),
                [&active.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "{table}");
    }
}

#[test]
fn permanent_deletion_rolls_back_dependencies_and_revision_on_failure() {
    let (_directory, database) = database();
    let mut value = draft("keep on failure");
    value.subtasks.push(SubtaskDraft {
        id: None,
        title: "keep child".into(),
        completed: false,
    });
    let active = find(&database.create_task(value).unwrap());
    let before = database.task_snapshot().unwrap();
    database.connection.lock().unwrap().execute_batch(
        "CREATE TRIGGER block_task_delete BEFORE DELETE ON tasks BEGIN SELECT RAISE(ABORT, 'simulated failure'); END;"
    ).unwrap();
    assert!(database
        .purge_task(&active.id, active.version, true)
        .is_err());
    let after = database.task_snapshot().unwrap();
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.tasks[0].version, active.version);
    assert_eq!(after.tasks[0].subtasks.len(), 1);
    let count: i64 = database
        .connection
        .lock()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM task_history WHERE task_id=?1",
            [&active.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}
fn draft(title: &str) -> TaskDraft {
    TaskDraft {
        title: title.into(),
        ..Default::default()
    }
}
fn find(result: &MutationResult) -> Task {
    result
        .snapshot
        .tasks
        .iter()
        .find(|task| Some(&task.id) == result.task_id.as_ref())
        .unwrap()
        .clone()
}
fn edited(task: &Task) -> TaskDraft {
    TaskDraft {
        schedule: task.schedule.as_ref().map(|s| s.config.clone()),
        repeat_scope: Default::default(),
        title: task.title.clone(),
        description: task.description.clone(),
        priority: task.priority,
        category_id: task.category_id.clone(),
        planned_date: task.planned_date.clone(),
        due_date: task.due_date.clone(),
        due_at: task.due_at.clone(),
        sort_order: task.sort_order,
        subtasks: task
            .subtasks
            .iter()
            .map(|item| SubtaskDraft {
                id: Some(item.id.clone()),
                title: item.title.clone(),
                completed: item.completed_at.is_some(),
            })
            .collect(),
    }
}

#[test]
fn task_lifecycle_audit_and_persistence() {
    let (directory, database) = database();
    let result = database.create_task(draft("  收集想法  ")).unwrap();
    let mut task = find(&result);
    assert_eq!(task.title, "收集想法");
    assert!(task.planned_date.is_none());
    assert!(uuid::Uuid::parse_str(&task.id).is_ok());
    let mut change = edited(&task);
    change.title = "安排今天".into();
    change.planned_date = Some("2026-10-28".into());
    change.due_date = Some("2026-11-01".into());
    change.priority = Priority::High;
    task = find(
        &database
            .update_task(&task.id, task.version, change)
            .unwrap(),
    );
    task = find(
        &database
            .set_task_status(&task.id, task.version, TaskStatus::Completed, false)
            .unwrap(),
    );
    let completed = task.completed_at.clone();
    assert!(completed.is_some());
    task = find(
        &database
            .set_task_deleted(&task.id, task.version, true)
            .unwrap(),
    );
    assert!(task.deleted_at.is_some());
    task = find(
        &database
            .set_task_deleted(&task.id, task.version, false)
            .unwrap(),
    );
    assert!(task.deleted_at.is_none());
    assert_eq!(task.completed_at, completed);
    task = find(
        &database
            .set_task_status(&task.id, task.version, TaskStatus::Pending, false)
            .unwrap(),
    );
    assert!(task.completed_at.is_none());
    let connection = database.connection.lock().unwrap();
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM task_history WHERE task_id=?1",
            [&task.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 6);
    let after: String = connection
        .query_row(
            "SELECT after_json FROM task_history WHERE operation='update'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&after).unwrap()["title"],
        "安排今天"
    );
    drop(connection);
    drop(database);
    let reopened = Database::open(&directory.path().join("tasks.sqlite3")).unwrap();
    let snapshot = reopened.task_snapshot().unwrap();
    assert_eq!(snapshot.revision, 6);
    assert_eq!(
        snapshot.tasks[0].planned_date.as_deref(),
        Some("2026-10-28")
    );
    assert_eq!(snapshot.tasks[0].due_date.as_deref(), Some("2026-11-01"));
}

#[test]
fn subtask_progress_sorting_and_parent_confirmation() {
    let (_dir, database) = database();
    let mut change = draft("父任务");
    change.subtasks = vec![
        SubtaskDraft {
            id: None,
            title: "第一步".into(),
            completed: false,
        },
        SubtaskDraft {
            id: None,
            title: "第二步".into(),
            completed: true,
        },
    ];
    let mut task = find(&database.create_task(change).unwrap());
    assert_eq!(
        task.progress,
        Progress {
            total: 2,
            completed: 1,
            percent: 50
        }
    );
    assert!(matches!(
        database.set_task_status(&task.id, task.version, TaskStatus::Completed, false),
        Err(ServiceError::ConfirmIncomplete)
    ));
    assert_eq!(database.task_snapshot().unwrap().revision, 1);
    task = find(
        &database
            .set_task_status(&task.id, task.version, TaskStatus::Completed, true)
            .unwrap(),
    );
    assert!(task.subtasks[0].completed_at.is_none());
    task = find(
        &database
            .set_task_status(&task.id, task.version, TaskStatus::Pending, false)
            .unwrap(),
    );
    let mut change = edited(&task);
    change.subtasks.reverse();
    change.subtasks[1].completed = true;
    task = find(
        &database
            .update_task(&task.id, task.version, change)
            .unwrap(),
    );
    assert_eq!(task.progress.percent, 100);
    assert_eq!(task.status, TaskStatus::Pending);
    assert_eq!(task.subtasks[0].title, "第二步");
    let mut change = edited(&task);
    change.subtasks.remove(0);
    change.subtasks[0].title = "修改后".into();
    task = find(
        &database
            .update_task(&task.id, task.version, change)
            .unwrap(),
    );
    assert_eq!(task.progress.total, 1);
    assert_eq!(task.subtasks[0].sort_order, 0);
    assert_eq!(task.subtasks[0].title, "修改后");
}

#[test]
fn category_deletion_preserves_tasks_and_versions_them() {
    let (_dir, database) = database();
    let result = database
        .save_category(CategoryDraft {
            id: None,
            expected_version: None,
            name: "工作".into(),
            color: "#6358d4".into(),
            sort_order: 0,
        })
        .unwrap();
    let category = result.snapshot.categories[0].clone();
    let mut change = draft("保留的任务");
    change.category_id = Some(category.id.clone());
    let task = find(&database.create_task(change).unwrap());
    let result = database
        .delete_category(&category.id, category.version)
        .unwrap();
    assert!(result.snapshot.categories.is_empty());
    assert_eq!(result.snapshot.tasks.len(), 1);
    assert!(result.snapshot.tasks[0].category_id.is_none());
    assert_eq!(result.snapshot.tasks[0].version, task.version + 1);
    assert!(matches!(
        database.update_task(&task.id, task.version, edited(&task)),
        Err(ServiceError::Conflict)
    ));
    let conn = database.connection.lock().unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM task_history WHERE revision=3",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn validation_rejects_bad_dates_deadlines_foreign_keys_and_subtask_theft() {
    let (_dir, database) = database();
    for date in [
        "2026-02-29",
        "2026-04-31",
        "2026-13-01",
        "2026-1-01",
        "0000-01-01",
    ] {
        let mut change = draft("日期");
        change.planned_date = Some(date.into());
        assert!(database.create_task(change).is_err());
    }
    assert!(database.create_task(draft("　 ")).is_err());
    let mut change = draft("错误分类");
    change.category_id = Some(uuid::Uuid::new_v4().to_string());
    assert!(matches!(
        database.create_task(change),
        Err(ServiceError::NotFound)
    ));
    let mut change = draft("双截止");
    change.due_date = Some("2026-10-28".into());
    change.due_at = Some("2026-10-28T08:00:00+08:00".into());
    assert!(database.create_task(change).is_err());
    let mut change = draft("UTC 时间");
    change.due_at = Some("2026-10-28T08:00:00+08:00".into());
    let task = find(&database.create_task(change).unwrap());
    assert_eq!(task.due_at.as_deref(), Some("2026-10-28T00:00:00.000Z"));
    let mut change = edited(&task);
    change.subtasks = vec![SubtaskDraft {
        id: Some(uuid::Uuid::new_v4().to_string()),
        title: "盗用".into(),
        completed: false,
    }];
    assert!(database
        .update_task(&task.id, task.version, change)
        .is_err());
    assert_eq!(database.task_snapshot().unwrap().revision, 1);
    let conn = database.connection.lock().unwrap();
    assert!(conn
        .execute("UPDATE tasks SET planned_date='2026-02-30'", [])
        .is_err());
    assert!(conn
        .execute("UPDATE tasks SET status='invalid'", [])
        .is_err());
    assert!(conn
        .execute("UPDATE tasks SET category_id='missing'", [])
        .is_err());
}

#[test]
fn audit_failure_rolls_back_task_and_revision() {
    let (_dir, database) = database();
    database.connection.lock().unwrap().execute_batch("CREATE TRIGGER test_audit_failure BEFORE INSERT ON task_history BEGIN SELECT RAISE(ABORT,'test-only audit failure'); END;").unwrap();
    assert!(database.create_task(draft("不能只保存任务")).is_err());
    let snapshot = database.task_snapshot().unwrap();
    assert!(snapshot.tasks.is_empty());
    assert_eq!(snapshot.revision, 0);
}

#[test]
fn concurrent_connections_do_not_silently_overwrite() {
    let (dir, database) = database();
    let initial = find(&database.create_task(draft("初始")).unwrap());
    let path = dir.path().join("tasks.sqlite3");
    let other = Database::open(&path).unwrap();
    let id = initial.id.clone();
    let version = initial.version;
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let other_barrier = barrier.clone();
    let writer = std::thread::spawn(move || {
        other_barrier.wait();
        other.update_task(&id, version, draft("窗口二"))
    });
    barrier.wait();
    let result = database.update_task(&initial.id, initial.version, draft("窗口一"));
    let other = writer.join().unwrap();
    assert_eq!(usize::from(result.is_ok()) + usize::from(other.is_ok()), 1);
    assert!(
        matches!(result, Err(ServiceError::Conflict))
            || matches!(other, Err(ServiceError::Conflict))
    );
    let snapshot = database.task_snapshot().unwrap();
    assert_eq!(snapshot.revision, 2);
    assert_eq!(snapshot.tasks[0].version, 2);
}

#[test]
fn rapid_independent_writes_have_unique_ids_and_contiguous_audit_revisions() {
    let (dir, database) = database();
    let path = dir.path().join("tasks.sqlite3");
    let other = Database::open(&path).unwrap();
    let worker = std::thread::spawn(move || {
        for _ in 0..15 {
            other.create_task(draft("窗口二连续写入")).unwrap();
        }
    });
    for _ in 0..15 {
        database.create_task(draft("窗口一连续写入")).unwrap();
    }
    worker.join().unwrap();
    let snapshot = database.task_snapshot().unwrap();
    assert_eq!(snapshot.revision, 30);
    assert_eq!(snapshot.tasks.len(), 30);
    let ids: std::collections::HashSet<_> = snapshot.tasks.iter().map(|task| &task.id).collect();
    assert_eq!(ids.len(), 30);
    let conn = database.connection.lock().unwrap();
    let revisions: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT revision) FROM task_history",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(revisions, 30);
}

#[test]
fn subtask_insert_failure_rolls_back_parent_update_and_deleted_children() {
    let (_dir, database) = database();
    let mut value = draft("原始标题");
    value.subtasks = vec![SubtaskDraft {
        id: None,
        title: "原始子任务".into(),
        completed: false,
    }];
    let original = find(&database.create_task(value).unwrap());
    database.connection.lock().unwrap().execute_batch("CREATE TRIGGER test_subtask_failure BEFORE INSERT ON subtasks WHEN NEW.title='拒绝' BEGIN SELECT RAISE(ABORT,'test-only subtask failure'); END;").unwrap();
    let mut change = edited(&original);
    change.title = "不应保存".into();
    change.subtasks[0].title = "拒绝".into();
    assert!(database
        .update_task(&original.id, original.version, change)
        .is_err());
    let snapshot = database.task_snapshot().unwrap();
    assert_eq!(snapshot.revision, 1);
    assert_eq!(snapshot.tasks[0].title, "原始标题");
    assert_eq!(snapshot.tasks[0].subtasks[0].id, original.subtasks[0].id);
    assert_eq!(snapshot.tasks[0].subtasks[0].title, "原始子任务");
}

#[test]
fn category_edit_duplicate_name_and_invalid_color_are_transactional() {
    let (_dir, database) = database();
    let result = database
        .save_category(CategoryDraft {
            id: None,
            expected_version: None,
            name: "Work".into(),
            color: "#6558d4".into(),
            sort_order: 0,
        })
        .unwrap();
    let category = result.snapshot.categories[0].clone();
    assert!(matches!(
        database.save_category(CategoryDraft {
            id: None,
            expected_version: None,
            name: "work".into(),
            color: "#6558d4".into(),
            sort_order: 0
        }),
        Err(ServiceError::DuplicateCategory)
    ));
    assert!(database
        .save_category(CategoryDraft {
            id: None,
            expected_version: None,
            name: "new".into(),
            color: "red".into(),
            sort_order: 0
        })
        .is_err());
    let result = database
        .save_category(CategoryDraft {
            id: Some(category.id.clone()),
            expected_version: Some(category.version),
            name: "个人".into(),
            color: "#118844".into(),
            sort_order: 1,
        })
        .unwrap();
    assert_eq!(result.snapshot.revision, 2);
    assert_eq!(result.snapshot.categories[0].version, 2);
    assert_eq!(result.snapshot.categories[0].name, "个人");
}
