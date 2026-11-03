use super::{validation, *};
use crate::{db::Database, error::ServiceError};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::Serialize;
use std::collections::HashMap;

/// Used only inside the existing task mutation transaction. Generation and
/// reminder transitions share its revision and durable audit boundary.
pub(crate) fn schedule_audit(
    tx: &Transaction<'_>,
    id: &str,
    operation: &str,
    before: Option<&Task>,
) -> Result<(), ServiceError> {
    let change = Change {
        revision: tx.query_row("SELECT revision FROM task_state WHERE id=1", [], |r| {
            r.get(0)
        })?,
        transaction_id: validation::id(),
        at: validation::now(),
    };
    audit(tx, &change, "task", id, operation, before, &task(tx, id)?)
}
pub(crate) fn insert_scheduled(
    tx: &Transaction<'_>,
    draft: TaskDraft,
) -> Result<String, ServiceError> {
    let draft = validation::draft(draft)?;
    let id = validation::id();
    let now = validation::now();
    let order: i64 = tx.query_row(
        "SELECT COALESCE(MAX(sort_order),-1024)+1024 FROM tasks",
        [],
        |r| r.get(0),
    )?;
    if order > 9_007_199_254_740_991 {
        return Err(ServiceError::Validation("排序值超出范围。"));
    }
    tx.execute("INSERT INTO tasks(id,title,description,priority,category_id,planned_date,due_date,due_at,created_at,updated_at,sort_order) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?9,?10)",params![id,draft.title,draft.description,draft.priority.as_str(),draft.category_id,draft.planned_date,draft.due_date,draft.due_at,now,order])?;
    let change = Change {
        revision: 0,
        transaction_id: validation::id(),
        at: now,
    };
    replace_subtasks(tx, &change, &id, &draft.subtasks, &[])?;
    crate::schedule::repository::save_generated(tx, &id, &draft)?;
    Ok(id)
}

pub(crate) struct Change {
    revision: i64,
    transaction_id: String,
    at: String,
}

pub(crate) fn reminder_audit(
    tx: &Transaction<'_>,
    id: &str,
    operation: &str,
) -> Result<(), ServiceError> {
    let change = Change {
        revision: tx.query_row("SELECT revision FROM task_state WHERE id=1", [], |r| {
            r.get(0)
        })?,
        transaction_id: validation::id(),
        at: validation::now(),
    };
    let mut q = tx.prepare("SELECT id,slot,scheduled_at,state FROM reminders WHERE task_id=?1")?;
    let rows=q.query_map([id],|r|Ok(serde_json::json!({"id":r.get::<_,String>(0)?,"slot":r.get::<_,String>(1)?,"scheduledAt":r.get::<_,String>(2)?,"state":r.get::<_,String>(3)?})))?.collect::<Result<Vec<_>,_>>()?;
    audit::<serde_json::Value, _>(
        tx,
        &change,
        "task",
        id,
        operation,
        None,
        &serde_json::json!({"taskId":id,"reminders":rows}),
    )
}

pub(crate) fn read_tasks(connection: &Connection) -> Result<Vec<Task>, ServiceError> {
    read_filtered(connection, None)
}
pub(crate) fn read_selected(
    connection: &Connection,
    ids: &[String],
) -> Result<Vec<Task>, ServiceError> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    if ids.len() > 1000 {
        return Err(ServiceError::Validation("单次读取任务过多。"));
    }
    let mut tasks = read_filtered(connection, Some(ids))?;
    tasks.sort_by_key(|t| ids.iter().position(|id| id == &t.id).unwrap_or(usize::MAX));
    Ok(tasks)
}
fn read_filtered(
    connection: &Connection,
    ids: Option<&[String]>,
) -> Result<Vec<Task>, ServiceError> {
    let values = ids.unwrap_or(&[]);
    let placeholders = std::iter::repeat_n("?", values.len())
        .collect::<Vec<_>>()
        .join(",");
    let restriction = if ids.is_some() {
        format!(" WHERE id IN ({placeholders})")
    } else {
        String::new()
    };

    let sql=format!("SELECT id,title,description,status,priority,category_id,planned_date,due_date,due_at,created_at,updated_at,completed_at,deleted_at,sort_order,version FROM tasks{restriction} ORDER BY sort_order,created_at,id");
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(rusqlite::params_from_iter(values.iter()), |row| {
        let status: String = row.get(3)?;
        let priority: String = row.get(4)?;
        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            status: match status.as_str() {
                "pending" => TaskStatus::Pending,
                "completed" => TaskStatus::Completed,
                _ => return Err(rusqlite::Error::InvalidQuery),
            },
            priority: match priority.as_str() {
                "none" => Priority::None,
                "low" => Priority::Low,
                "medium" => Priority::Medium,
                "high" => Priority::High,
                _ => return Err(rusqlite::Error::InvalidQuery),
            },
            category_id: row.get(5)?,
            planned_date: row.get(6)?,
            due_date: row.get(7)?,
            due_at: row.get(8)?,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
            completed_at: row.get(11)?,
            deleted_at: row.get(12)?,
            sort_order: row.get(13)?,
            version: row.get(14)?,
            schedule: None,
            subtasks: vec![],
            progress: Progress {
                total: 0,
                completed: 0,
                percent: 0,
            },
        })
    })?;
    let mut tasks = rows.collect::<Result<Vec<_>, _>>()?;
    let restriction = if ids.is_some() {
        format!(" WHERE task_id IN ({placeholders})")
    } else {
        String::new()
    };
    let sql=format!("SELECT task_id,id,title,completed_at,sort_order,created_at,updated_at FROM subtasks{restriction} ORDER BY task_id,sort_order,id");
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(rusqlite::params_from_iter(values.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            Subtask {
                id: row.get(1)?,
                title: row.get(2)?,
                completed_at: row.get(3)?,
                sort_order: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            },
        ))
    })?;
    let mut subtasks: HashMap<String, Vec<Subtask>> = HashMap::new();
    for item in rows {
        let (id, item) = item?;
        subtasks.entry(id).or_default().push(item);
    }
    for task in &mut tasks {
        task.subtasks = subtasks.remove(&task.id).unwrap_or_default();
        task.calculate_progress();
        task.schedule = crate::schedule::repository::info(connection, &task.id)?;
    }
    Ok(tasks)
}

fn read_categories(connection: &Connection) -> Result<Vec<Category>, ServiceError> {
    let mut statement = connection.prepare("SELECT id,name,color,sort_order,version,created_at,updated_at FROM categories ORDER BY sort_order,name,id")?;
    let rows = statement.query_map([], |row| {
        Ok(Category {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            sort_order: row.get(3)?,
            version: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn snapshot(connection: &Connection) -> Result<TaskSnapshot, ServiceError> {
    Ok(TaskSnapshot {
        revision: connection.query_row(
            "SELECT revision FROM task_state WHERE id=1",
            [],
            |row| row.get(0),
        )?,
        tasks: read_tasks(connection)?,
        categories: read_categories(connection)?,
        reminders: crate::schedule::repository::reminders(connection)?,
    })
}
fn task(connection: &Connection, id: &str) -> Result<Task, ServiceError> {
    read_tasks(connection)?
        .into_iter()
        .find(|task| task.id == id)
        .ok_or(ServiceError::NotFound)
}
fn category(connection: &Connection, id: &str) -> Result<Category, ServiceError> {
    read_categories(connection)?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or(ServiceError::NotFound)
}
fn current_task(
    connection: &Connection,
    id: &str,
    expected_version: i64,
    allow_deleted: bool,
) -> Result<Task, ServiceError> {
    validation::validate_id(id)?;
    validation::version(expected_version)?;
    let task = task(connection, id)?;
    if !allow_deleted && task.deleted_at.is_some() {
        return Err(ServiceError::NotFound);
    }
    if task.version != expected_version {
        return Err(ServiceError::Conflict);
    }
    Ok(task)
}
fn valid_category(connection: &Connection, id: &Option<String>) -> Result<(), ServiceError> {
    if let Some(id) = id {
        category(connection, id)?;
    }
    Ok(())
}

fn audit<B: Serialize, A: Serialize>(
    transaction: &Transaction<'_>,
    change: &Change,
    entity_type: &str,
    id: &str,
    operation: &str,
    before: Option<&B>,
    after: &A,
) -> Result<(), ServiceError> {
    let before = before.map(serde_json::to_string).transpose()?;
    let after = serde_json::to_string(after)?;
    transaction.execute("INSERT INTO task_history (id,transaction_id,revision,entity_type,entity_id,task_id,operation,before_json,after_json,occurred_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![validation::id(), change.transaction_id, change.revision, entity_type, id, if entity_type == "task" {Some(id)} else {None}, operation, before, after, change.at])?;
    Ok(())
}

fn replace_subtasks(
    transaction: &Transaction<'_>,
    change: &Change,
    task_id: &str,
    drafts: &[SubtaskDraft],
    previous: &[Subtask],
) -> Result<(), ServiceError> {
    for draft in drafts {
        if let Some(id) = &draft.id {
            if !previous.iter().any(|item| item.id == *id) {
                return Err(ServiceError::Validation(
                    "子任务不属于当前任务，请重新加载。",
                ));
            }
        }
    }
    transaction.execute("DELETE FROM subtasks WHERE task_id=?1", [task_id])?;
    for (position, draft) in drafts.iter().enumerate() {
        let old = draft
            .id
            .as_ref()
            .and_then(|id| previous.iter().find(|item| item.id == *id));
        let id = old
            .map(|item| item.id.clone())
            .unwrap_or_else(validation::id);
        let completed_at = if draft.completed {
            old.and_then(|item| item.completed_at.clone())
                .or_else(|| Some(change.at.clone()))
        } else {
            None
        };
        let created_at = old
            .map(|item| item.created_at.as_str())
            .unwrap_or(&change.at);
        let unchanged = old.is_some_and(|item| {
            item.title == draft.title
                && item.completed_at == completed_at
                && item.sort_order == position as i64
        });
        let updated_at = if unchanged {
            &old.expect("unchanged subtask exists").updated_at
        } else {
            &change.at
        };
        transaction.execute("INSERT INTO subtasks (id,task_id,title,completed_at,sort_order,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![id, task_id, draft.title, completed_at, position as i64, created_at, updated_at])?;
    }
    Ok(())
}

impl Database {
    pub fn task_snapshot(&self) -> Result<TaskSnapshot, ServiceError> {
        let mut connection = self.connection.lock().map_err(|_| ServiceError::Lock)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Deferred)?;
        let result = snapshot(&transaction)?;
        transaction.commit()?;
        Ok(result)
    }

    pub(crate) fn mutate_tasks(
        &self,
        operation: impl FnOnce(
            &Transaction<'_>,
            &Change,
        ) -> Result<(Option<String>, Option<String>), ServiceError>,
    ) -> Result<MutationResult, ServiceError> {
        let mut connection = self.connection.lock().map_err(|_| ServiceError::Lock)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if transaction.execute("UPDATE task_state SET revision=revision+1 WHERE id=1", [])? != 1 {
            return Err(ServiceError::InvalidSettings);
        }
        let change = Change {
            revision: transaction.query_row(
                "SELECT revision FROM task_state WHERE id=1",
                [],
                |row| row.get(0),
            )?,
            transaction_id: validation::id(),
            at: validation::now(),
        };
        let (task_id, category_id) = operation(&transaction, &change)?;
        crate::schedule::repository::cancel_inactive(&transaction)?;
        let snapshot = snapshot(&transaction)?;
        transaction.commit()?;
        Ok(MutationResult {
            snapshot,
            task_id,
            category_id,
        })
    }

    pub fn create_task(&self, draft: TaskDraft) -> Result<MutationResult, ServiceError> {
        let draft = validation::draft(draft)?;
        self.mutate_tasks(|transaction, change| {
            valid_category(transaction, &draft.category_id)?;
            let id = validation::id();
            let sort_order: i64 = transaction.query_row("SELECT COALESCE(MAX(sort_order),-1024)+1024 FROM tasks", [], |row| row.get(0))?;
            if sort_order > 9_007_199_254_740_991 { return Err(ServiceError::Validation("排序值超出范围。")); }
            transaction.execute("INSERT INTO tasks (id,title,description,priority,category_id,planned_date,due_date,due_at,created_at,updated_at,sort_order) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9,?10)",
                params![id, draft.title, draft.description, draft.priority.as_str(), draft.category_id, draft.planned_date, draft.due_date, draft.due_at, change.at, sort_order])?;
            replace_subtasks(transaction, change, &id, &draft.subtasks, &[])?;
            crate::schedule::repository::save(transaction, &id, &draft, None)?;
            let after = task(transaction, &id)?;
            audit::<Task, Task>(transaction, change, "task", &id, "create", None, &after)?;
            Ok((Some(id), None))
        })
    }

    pub fn update_task(
        &self,
        id: &str,
        expected_version: i64,
        draft: TaskDraft,
    ) -> Result<MutationResult, ServiceError> {
        let draft = validation::draft(draft)?;
        self.mutate_tasks(|transaction, change| {
            let before = current_task(transaction, id, expected_version, false)?;
            valid_category(transaction, &draft.category_id)?;
            transaction.execute("UPDATE tasks SET title=?1,description=?2,priority=?3,category_id=?4,planned_date=?5,due_date=?6,due_at=?7,sort_order=?8,updated_at=?9,version=version+1 WHERE id=?10",
                params![draft.title,draft.description,draft.priority.as_str(),draft.category_id,draft.planned_date,draft.due_date,draft.due_at,draft.sort_order,change.at,id])?;
            replace_subtasks(transaction, change, id, &draft.subtasks, &before.subtasks)?;
            crate::schedule::repository::save(transaction, id, &draft, Some(&before))?;
            audit(transaction, change, "task", id, "update", Some(&before), &task(transaction,id)?)?;
            Ok((Some(id.to_string()), None))
        })
    }

    pub fn set_task_status(
        &self,
        id: &str,
        expected_version: i64,
        status: TaskStatus,
        confirm_incomplete: bool,
    ) -> Result<MutationResult, ServiceError> {
        self.mutate_tasks(|transaction, change| {
            let before = current_task(transaction,id,expected_version,false)?;
            if status == TaskStatus::Completed && before.status != TaskStatus::Completed && before.progress.completed < before.progress.total && !confirm_incomplete {
                return Err(ServiceError::ConfirmIncomplete);
            }
            let reopening = status == TaskStatus::Pending && before.status == TaskStatus::Completed;
            let completed_at = if status == TaskStatus::Completed { before.completed_at.clone().or_else(|| Some(change.at.clone())) } else { None };
            transaction.execute("UPDATE tasks SET status=?1,completed_at=?2,updated_at=?3,version=version+1 WHERE id=?4", params![status.as_str(), completed_at, change.at, id])?;
            if reopening {
                crate::schedule::repository::resume_periodic(transaction, id)?;
            }
            audit(transaction, change, "task", id, if status == TaskStatus::Completed { "complete" } else { "reopen" }, Some(&before), &task(transaction,id)?)?;
            Ok((Some(id.to_string()), None))
        })
    }

    #[cfg(test)]
    pub fn set_task_deleted(
        &self,
        id: &str,
        expected_version: i64,
        deleted: bool,
    ) -> Result<MutationResult, ServiceError> {
        self.mutate_tasks(|transaction, change| {
            let before = current_task(transaction, id, expected_version, true)?;
            transaction.execute(
                "UPDATE tasks SET deleted_at=?1,updated_at=?2,version=version+1 WHERE id=?3",
                params![if deleted { Some(&change.at) } else { None }, change.at, id],
            )?;
            audit(
                transaction,
                change,
                "task",
                id,
                if deleted { "delete" } else { "restore" },
                Some(&before),
                &task(transaction, id)?,
            )?;
            Ok((Some(id.to_string()), None))
        })
    }

    pub fn purge_task(
        &self,
        id: &str,
        expected_version: i64,
        confirmed: bool,
    ) -> Result<MutationResult, ServiceError> {
        if !confirmed {
            return Err(ServiceError::Validation("彻底删除任务需要明确确认。"));
        }
        self.mutate_tasks(|transaction, _change| {
            current_task(transaction, id, expected_version, true)?;
            // Keep the occurrence exclusion even after removing its task_schedule
            // key, otherwise a rewound/split recurrence cursor could recreate it.
            let occurrence: Option<(String, String)> = transaction
                .query_row(
                    "SELECT series_id,occurrence_date FROM task_schedule WHERE task_id=?1 AND series_id IS NOT NULL AND occurrence_date IS NOT NULL",
                    [id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            if let Some((series_id, date)) = occurrence {
                let json: String = transaction.query_row(
                    "SELECT exclusions_json FROM recurrence_series WHERE id=?1",
                    [&series_id],
                    |row| row.get(0),
                )?;
                let mut exclusions: Vec<String> = serde_json::from_str(&json)?;
                if !exclusions.contains(&date) {
                    exclusions.push(date);
                    transaction.execute(
                        "UPDATE recurrence_series SET exclusions_json=?1,version=version+1 WHERE id=?2",
                        params![serde_json::to_string(&exclusions)?, series_id],
                    )?;
                }
            }
            // Explicitly remove RESTRICT references before the task itself.
            // Purging also removes its audit content; no new task-history row
            // can reference the removed task. The shared revision still advances.
            transaction.execute("DELETE FROM reminders WHERE task_id=?1", [id])?;
            transaction.execute("DELETE FROM task_schedule WHERE task_id=?1", [id])?;
            transaction.execute("DELETE FROM subtasks WHERE task_id=?1", [id])?;
            transaction.execute(
                "DELETE FROM task_history WHERE task_id=?1 OR (entity_type='task' AND entity_id=?1)",
                [id],
            )?;
            transaction.execute("DELETE FROM tasks WHERE id=?1", [id])?;
            // Keep the notification delivery ledger to prevent historical toasts
            // being resent when an older backup is explicitly restored later.
            Ok((Some(id.to_string()), None))
        })
    }

    pub fn save_category(&self, draft: CategoryDraft) -> Result<MutationResult, ServiceError> {
        let name = validation::text(&draft.name, 100)?;
        validation::color(&draft.color)?;
        if !(-9_007_199_254_740_991..=9_007_199_254_740_991).contains(&draft.sort_order) {
            return Err(ServiceError::Validation("排序值超出范围。"));
        }
        self.mutate_tasks(|transaction,change| {
            let id = draft.id.clone().unwrap_or_else(validation::id);
            validation::validate_id(&id)?;
            let before = if draft.id.is_some() {
                let before = category(transaction,&id)?;
                let expected = draft.expected_version.ok_or(ServiceError::Conflict)?;
                validation::version(expected)?;
                if before.version != expected { return Err(ServiceError::Conflict); }
                Some(before)
            } else { None };
            let duplicate: Option<String> = transaction.query_row("SELECT id FROM categories WHERE name=?1 COLLATE NOCASE AND id<>?2", params![name,id], |row|row.get(0)).optional()?;
            if duplicate.is_some() { return Err(ServiceError::DuplicateCategory); }
            if before.is_some() {
                transaction.execute("UPDATE categories SET name=?1,color=?2,sort_order=?3,version=version+1,updated_at=?4 WHERE id=?5",params![name,draft.color,draft.sort_order,change.at,id])?;
            } else {
                transaction.execute("INSERT INTO categories (id,name,color,sort_order,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?5)",params![id,name,draft.color,draft.sort_order,change.at])?;
            }
            audit(transaction,change,"category",&id,if before.is_some(){"category_update"}else{"category_create"},before.as_ref(),&category(transaction,&id)?)?;
            Ok((None,Some(id)))
        })
    }

    pub fn delete_category(
        &self,
        id: &str,
        expected_version: i64,
    ) -> Result<MutationResult, ServiceError> {
        validation::validate_id(id)?;
        validation::version(expected_version)?;
        self.mutate_tasks(|transaction, change| {
            let before = category(transaction, id)?;
            if before.version != expected_version {
                return Err(ServiceError::Conflict);
            }
            let affected = read_tasks(transaction)?
                .into_iter()
                .filter(|task| task.category_id.as_deref() == Some(id))
                .collect::<Vec<_>>();
            for before in affected {
                transaction.execute(
                    "UPDATE tasks SET category_id=NULL,updated_at=?1,version=version+1 WHERE id=?2",
                    params![change.at, before.id],
                )?;
                audit(
                    transaction,
                    change,
                    "task",
                    &before.id,
                    "category_unlink",
                    Some(&before),
                    &task(transaction, &before.id)?,
                )?;
            }
            transaction.execute("DELETE FROM categories WHERE id=?1", [id])?;
            audit(
                transaction,
                change,
                "category",
                id,
                "category_delete",
                Some(&before),
                &serde_json::json!({"id":id,"deleted":true}),
            )?;
            Ok((None, Some(id.to_string())))
        })
    }
}
