use super::{calendar::*, model::*};
use crate::{
    error::ServiceError,
    tasks::{self, validation, Task, TaskDraft, TaskStatus},
};
use chrono::{Duration, Local, NaiveDate, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

// Allows old-schema migration fixtures to read tasks before applying 0004.
// Production always applies all migrations before exposing LocalService.
fn has_schema(c: &Connection) -> Result<bool, ServiceError> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='task_schedule')",
        [],
        |r| r.get(0),
    )?)
}

pub fn info(c: &Connection, id: &str) -> Result<Option<ScheduleInfo>, ServiceError> {
    if !has_schema(c)? {
        return Ok(None);
    }
    let row: Option<(String, Option<String>, Option<String>)> = c
        .query_row(
            "SELECT config_json,series_id,occurrence_date FROM task_schedule WHERE task_id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    row.map(|(json, series_id, occurrence_date)| {
        Ok(ScheduleInfo {
            config: serde_json::from_str(&json)?,
            series_id,
            occurrence_date,
            next_at: c.query_row(
                "SELECT MIN(scheduled_at) FROM reminders WHERE task_id=?1 AND state='pending'",
                [id],
                |r| r.get(0),
            )?,
            has_notified: c.query_row(
                "SELECT EXISTS(SELECT 1 FROM reminders WHERE task_id=?1 AND state='notified')",
                [id],
                |r| r.get(0),
            )?,
        })
    })
    .transpose()
}
pub fn reminders(c: &Connection) -> Result<Vec<Reminder>, ServiceError> {
    if !has_schema(c)? {
        return Ok(vec![]);
    }
    let mut q=c.prepare("SELECT r.id,r.task_id,t.title,r.scheduled_at,r.state,r.channel FROM reminders r JOIN tasks t ON t.id=r.task_id WHERE t.deleted_at IS NULL AND t.status='pending' AND r.state IN('pending','notified') ORDER BY r.scheduled_at DESC")?;
    let rows = q
        .query_map([], |r| {
            Ok(Reminder {
                id: r.get(0)?,
                task_id: r.get(1)?,
                title: r.get(2)?,
                scheduled_at: r.get(3)?,
                state: r.get(4)?,
                channel: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
fn slots(
    tx: &Transaction<'_>,
    id: &str,
    config: &ScheduleConfig,
    due: Option<String>,
    rearm: bool,
) -> Result<(), ServiceError> {
    // Keep UUID/state for unchanged slots: title-only edits cannot resend a toast.
    let mut wanted: Vec<(String, String)> = vec![];
    if config.at.is_none() && config.before_minutes.is_empty() && !config.at_deadline {
        tx.execute("UPDATE reminders SET state='cancelled' WHERE task_id=?1 AND slot='snooze' AND state IN('pending','notified')",[id])?;
    }
    if let Some(at) = &config.at {
        if config.reminder_repeat.is_none() {
            wanted.push(("explicit".into(), stamp(instant(at)?)));
        }
    }
    if let Some(due) = &due {
        if config.at_deadline {
            wanted.push(("deadline".into(), due.clone()));
        }
        for minutes in &config.before_minutes {
            wanted.push((
                format!("before:{minutes}"),
                stamp(
                    instant(due)?
                        .checked_sub_signed(Duration::minutes(*minutes))
                        .ok_or(ServiceError::Validation("提醒时间超出范围。"))?,
                ),
            ));
        }
    }
    let mut q = tx.prepare(
        "SELECT slot,scheduled_at,state FROM reminders WHERE task_id=?1 AND slot<>'snooze' AND slot NOT GLOB 'periodic:*'",
    )?;
    let old = q
        .query_map([id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (slot, at, _) in &old {
        if !wanted.contains(&(slot.clone(), at.clone())) {
            tx.execute(
                "UPDATE reminders SET state='cancelled' WHERE task_id=?1 AND slot=?2",
                params![id, slot],
            )?;
        }
    }
    for (slot, at) in wanted {
        if old
            .iter()
            .any(|(s, t, state)| s == &slot && t == &at && (!rearm || state != "cancelled"))
        {
            continue;
        }
        tx.execute("INSERT INTO reminders(id,task_id,slot,scheduled_at,state) VALUES(?1,?2,?3,?4,'pending') ON CONFLICT(task_id,slot) DO UPDATE SET scheduled_at=excluded.scheduled_at,state='pending',channel=NULL,claimed_at=NULL,delivered_at=NULL",params![validation::id(),id,slot,at])?;
    }
    super::periodic::sync(tx, id, config, due.as_deref(), Utc::now())?;
    Ok(())
}
pub fn save(
    tx: &Transaction<'_>,
    id: &str,
    draft: &TaskDraft,
    before: Option<&Task>,
) -> Result<(), ServiceError> {
    save_inner(tx, id, draft, before, true)
}
/// Internal recurrence generation preserves old templates. Obsolete periodic
/// slots are cancelled by sync instead of blocking all generation/notifications.
pub(crate) fn save_generated(
    tx: &Transaction<'_>,
    id: &str,
    draft: &TaskDraft,
) -> Result<(), ServiceError> {
    save_inner(tx, id, draft, None, false)
}
fn save_inner(
    tx: &Transaction<'_>,
    id: &str,
    draft: &TaskDraft,
    before: Option<&Task>,
    strict_deadline: bool,
) -> Result<(), ServiceError> {
    if !has_schema(tx)? {
        return Ok(());
    }
    let previous = info(tx, id)?;
    let mut config = draft
        .schedule
        .clone()
        .or_else(|| previous.as_ref().map(|i| i.config.clone()))
        .unwrap_or_default();
    // Moving the completion date shifts an unchanged associated reminder anchor.
    if let Some(old) = before {
        if draft
            .schedule
            .as_ref()
            .is_none_or(|c| previous.as_ref().is_some_and(|p| p.config.at == c.at))
        {
            let old_day = completion_day(
                old.due_at.as_deref(),
                old.due_date.as_deref(),
                old.planned_date.as_deref(),
            )?;
            let new_day = completion_day(
                draft.due_at.as_deref(),
                draft.due_date.as_deref(),
                draft.planned_date.as_deref(),
            )?;
            if let (Some(a), Some(b), Some(at)) =
                (old_day.as_deref(), new_day.as_deref(), &config.at)
            {
                if a != b {
                    config.at = Some(shift_time(at, (date(b)? - date(a)?).num_days())?);
                }
            }
        }
    }
    let series_start: Option<String> = if draft.repeat_scope == EditScope::Once {
        previous
            .as_ref()
            .and_then(|s| s.series_id.as_deref())
            .map(|sid| {
                tx.query_row(
                    "SELECT start_date FROM recurrence_series WHERE id=?1",
                    [sid],
                    |r| r.get(0),
                )
            })
            .transpose()?
    } else {
        None
    };
    validate(
        &config,
        series_start.as_deref().or(draft.planned_date.as_deref()),
        draft.due_at.is_some() || draft.due_date.is_some() || draft.planned_date.is_some(),
    )?;
    let mut exclusions: Vec<String> = vec![];
    let mut series_id = previous.as_ref().and_then(|i| i.series_id.clone());
    let mut occurrence = previous.as_ref().and_then(|i| i.occurrence_date.clone());
    let recurrence_changed =
        previous.as_ref().map(|i| &i.config.recurrence) != Some(&config.recurrence);
    let following = draft.repeat_scope == EditScope::Following;
    if series_id.is_some() && following {
        // Split at this immutable occurrence key. Historical instances keep their series.
        let cutoff = occurrence.as_deref().ok_or(ServiceError::InvalidSettings)?;
        let inherited: String = tx.query_row(
            "SELECT exclusions_json FROM recurrence_series WHERE id=?1",
            [series_id.as_deref()],
            |r| r.get(0),
        )?;
        let inherited: Vec<String> = serde_json::from_str(&inherited)?;
        let mut q=tx.prepare("SELECT s.occurrence_date FROM task_schedule s JOIN tasks t ON t.id=s.task_id WHERE s.series_id=?1 AND s.occurrence_date>=?2 AND t.status='completed' AND s.task_id<>?3")?;
        exclusions = q
            .query_map(params![series_id, cutoff, id], |r| r.get::<_, String>(0))?
            .collect::<Result<_, _>>()?;
        exclusions.extend(inherited.into_iter().filter(|d| d.as_str() >= cutoff));
        exclusions.sort();
        exclusions.dedup();
        let end = date(cutoff)?
            .pred_opt()
            .ok_or(ServiceError::Validation("重复范围无效。"))?
            .to_string();
        tx.execute(
            "UPDATE recurrence_series SET end_date=?1,version=version+1 WHERE id=?2",
            params![end, series_id],
        )?;
        // Instances after cutoff can exist after editing an older occurrence. Keep
        // completed history; cancel old pending instances transactionally with audit.
        let affected = tasks::repository::read_tasks(tx)?
            .into_iter()
            .filter(|t| t.id != id && t.status == TaskStatus::Pending && t.deleted_at.is_none())
            .filter(|t| {
                t.schedule.as_ref().is_some_and(|s| {
                    s.series_id == series_id
                        && s.occurrence_date.as_deref().is_some_and(|d| d >= cutoff)
                })
            })
            .collect::<Vec<_>>();
        for old in affected {
            tx.execute(
                "UPDATE tasks SET deleted_at=?1,updated_at=?1,version=version+1 WHERE id=?2",
                params![validation::now(), old.id],
            )?;
            tasks::repository::schedule_audit(tx, &old.id, "recurrence_supersede", Some(&old))?;
        }
        // Detach this key from the old series while preserving every other key as a tombstone.
        series_id = None;
        occurrence = None;
    }
    if before.is_none() || following || (series_id.is_none() && recurrence_changed) {
        if let Some(rule) = &config.recurrence {
            let sid = validation::id();
            let start = draft
                .planned_date
                .as_deref()
                .ok_or(ServiceError::Validation("重复任务需要计划日期。"))?;
            let mut template = draft.clone();
            template.schedule = Some(config.clone());
            template.repeat_scope = EditScope::Once;
            for s in &mut template.subtasks {
                s.id = None;
                s.completed = false;
            }
            tx.execute("INSERT INTO recurrence_series(id,rule_json,template_json,start_date,cursor_date,end_date,exclusions_json) VALUES(?1,?2,?3,?4,?4,?5,?6)",params![sid,serde_json::to_string(rule)?,serde_json::to_string(&template)?,start,rule.until,serde_json::to_string(&exclusions)?])?;
            series_id = Some(sid);
            occurrence = Some(start.into());
        }
    }
    tx.execute("INSERT INTO task_schedule(task_id,config_json,series_id,occurrence_date) VALUES(?1,?2,?3,?4) ON CONFLICT(task_id) DO UPDATE SET config_json=excluded.config_json,series_id=excluded.series_id,occurrence_date=excluded.occurrence_date",params![id,serde_json::to_string(&config)?,series_id,occurrence])?;
    let due = effective_deadline(
        draft.due_at.as_deref(),
        draft.due_date.as_deref(),
        draft.planned_date.as_deref(),
    )?;
    if strict_deadline {
        super::periodic::validate_deadline(&config, due.as_deref())?;
    }
    let rearm = previous.as_ref().is_none_or(|p| p.config != config);
    slots(tx, id, &config, due, rearm)?;
    // Turning periodic reminders off must not replay their old first reminder
    // as a freshly-created one-shot reminder in the past.
    if previous.as_ref().is_some_and(|p| {
        p.config.reminder_repeat.is_some()
            && config.reminder_repeat.is_none()
            && p.config.at == config.at
    }) {
        if let Some(at) = &config.at {
            let at = instant(at)?;
            if at <= Utc::now() {
                tx.execute("UPDATE reminders SET state='dismissed' WHERE task_id=?1 AND slot='explicit' AND scheduled_at=?2 AND state='pending'",params![id,stamp(at)])?;
            }
        }
    }
    Ok(())
}
pub fn cancel_inactive(tx: &Transaction<'_>) -> Result<(), ServiceError> {
    if !has_schema(tx)? {
        return Ok(());
    }
    tx.execute("UPDATE reminders SET state='cancelled' WHERE state IN('pending','notified') AND task_id IN(SELECT id FROM tasks WHERE status<>'pending' OR deleted_at IS NOT NULL)",[])?;
    Ok(())
}
fn horizon(template: &TaskDraft) -> Result<NaiveDate, ServiceError> {
    let today = Local::now().date_naive();
    let start = date(
        template
            .planned_date
            .as_deref()
            .ok_or(ServiceError::InvalidSettings)?,
    )?;
    let mut earliest = start;
    if let Some(c) = &template.schedule {
        if let Some(at) = &c.at {
            earliest = earliest.min(instant(at)?.with_timezone(&Local).date_naive());
        }
        if c.at_deadline || !c.before_minutes.is_empty() {
            let due = template
                .due_at
                .as_deref()
                .map(|t| instant(t).map(|t| t.with_timezone(&Local).date_naive()))
                .transpose()?
                .or(template
                    .due_date
                    .as_deref()
                    .or(template.planned_date.as_deref())
                    .map(date)
                    .transpose()?);
            if let Some(d) = due {
                earliest = earliest.min(
                    d.checked_sub_signed(Duration::days(if c.before_minutes.contains(&1440) {
                        1
                    } else {
                        0
                    }))
                    .ok_or(ServiceError::Validation("提前提醒超出日期范围。"))?,
                );
            }
        }
    }
    let lead = 32.max((start - earliest).num_days() + 2);
    Ok(today
        .checked_add_signed(Duration::days(lead))
        .unwrap_or(NaiveDate::MAX)
        .min(NaiveDate::from_ymd_opt(9999, 12, 31).ok_or(ServiceError::InvalidSettings)?))
}
pub fn generation_needed(c: &Connection) -> Result<bool, ServiceError> {
    let mut q =
        c.prepare("SELECT cursor_date,template_json FROM recurrence_series WHERE active=1")?;
    for row in q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (cursor, template) = row?;
        if date(&cursor)? < horizon(&serde_json::from_str(&template)?)? {
            return Ok(true);
        }
    }
    Ok(false)
}
pub fn generate(tx: &Transaction<'_>) -> Result<(), ServiceError> {
    let mut q=tx.prepare("SELECT id,rule_json,template_json,start_date,cursor_date,end_date,exclusions_json FROM recurrence_series WHERE active=1 ORDER BY cursor_date,id")?;
    let rows = q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, String>(6)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut generated = 0;
    let mut processed = 0;
    for (sid, rule, template, start, cursor, end, exclusions) in rows {
        let exclusions: Vec<String> = serde_json::from_str(&exclusions)?;
        let rule: Recurrence = serde_json::from_str(&rule)?;
        let template: TaskDraft = serde_json::from_str(&template)?;
        let horizon = horizon(&template)?;
        let start = date(&start)?;
        let mut day = date(&cursor)?;
        if day >= horizon {
            continue;
        }
        processed += 1;
        for _ in 0..366 {
            let Some(next) = day.succ_opt() else { break };
            if next > horizon {
                break;
            }
            if end.as_ref().is_some_and(|s| next.to_string() > *s) {
                tx.execute("UPDATE recurrence_series SET active=0 WHERE id=?1", [&sid])?;
                break;
            }
            day = next;
            if matches(&rule, start, day) && !exclusions.contains(&day.to_string()) {
                let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM task_schedule WHERE series_id=?1 AND occurrence_date=?2)",params![sid,day.to_string()],|r|r.get(0))?;
                if !exists {
                    let mut draft = template.clone();
                    let days = (day - start).num_days();
                    draft.planned_date = Some(day.to_string());
                    draft.due_date = template
                        .due_date
                        .as_deref()
                        .map(|s| {
                            date(s)
                                .and_then(|d| {
                                    d.checked_add_signed(Duration::days(days))
                                        .ok_or(ServiceError::Validation("截止日期超出范围。"))
                                })
                                .map(|d| d.to_string())
                        })
                        .transpose()?;
                    draft.due_at = template
                        .due_at
                        .as_deref()
                        .map(|s| shift_time(s, days))
                        .transpose()?;
                    if let Some(c) = &mut draft.schedule {
                        c.at = c.at.as_deref().map(|s| shift_time(s, days)).transpose()?;
                        c.recurrence = None;
                    }
                    // Do not recreate a deleted category from a stored template.
                    if let Some(cat) = &draft.category_id {
                        let exists: bool = tx.query_row(
                            "SELECT EXISTS(SELECT 1 FROM categories WHERE id=?1)",
                            [cat],
                            |r| r.get(0),
                        )?;
                        if !exists {
                            draft.category_id = None;
                        }
                    }
                    let id = tasks::repository::insert_scheduled(tx, draft)?;
                    let mut c = info(tx, &id)?.ok_or(ServiceError::InvalidSettings)?.config;
                    c.recurrence = Some(rule.clone());
                    tx.execute("UPDATE task_schedule SET series_id=?1,occurrence_date=?2,config_json=?3 WHERE task_id=?4",params![sid,day.to_string(),serde_json::to_string(&c)?,id])?;
                    tasks::repository::schedule_audit(tx, &id, "recurrence_generate", None)?;
                    generated += 1;
                }
            }
            if generated >= 50 {
                break;
            }
        }
        tx.execute(
            "UPDATE recurrence_series SET cursor_date=?1 WHERE id=?2",
            params![day.to_string(), sid],
        )?;
        if generated >= 50 || processed >= 20 {
            break;
        }
    }
    Ok(())
}
pub fn claim(tx: &Transaction<'_>) -> Result<Vec<Reminder>, ServiceError> {
    cancel_inactive(tx)?;
    let now = Utc::now();
    super::periodic::advance_due(tx, now)?;
    let at = stamp(now);
    // Restore carries the delivery ledger forward, so regenerated occurrences
    // from an older snapshot cannot send a historical toast a second time.
    tx.execute("UPDATE reminders SET state='dismissed' WHERE state='pending' AND EXISTS(SELECT 1 FROM reminder_delivery_guard g LEFT JOIN task_schedule s ON s.task_id=reminders.task_id WHERE g.delivery_key=json_array(COALESCE(s.series_id,'task:'||reminders.task_id),COALESCE(s.occurrence_date,''),reminders.slot,reminders.scheduled_at))",[])?;

    tx.execute("UPDATE reminders SET state='notified',channel='unknown',claimed_at=?1 WHERE id IN(SELECT id FROM reminders WHERE state='pending' AND scheduled_at<=?1 ORDER BY scheduled_at LIMIT 500)",[&at])?;
    tx.execute("INSERT OR IGNORE INTO reminder_delivery_guard(delivery_key,claimed_at) SELECT json_array(COALESCE(s.series_id,'task:'||r.task_id),COALESCE(s.occurrence_date,''),r.slot,r.scheduled_at),r.claimed_at FROM reminders r LEFT JOIN task_schedule s ON s.task_id=r.task_id WHERE r.claimed_at=?1 AND r.state='notified'",[&at])?;
    Ok(reminders(tx)?
        .into_iter()
        .filter(|r| {
            r.state == "notified"
                && tx
                    .query_row(
                        "SELECT claimed_at=?1 FROM reminders WHERE id=?2",
                        params![at, r.id],
                        |r| r.get::<_, bool>(0),
                    )
                    .unwrap_or(false)
        })
        .collect())
}

pub fn resume_periodic(tx: &Transaction<'_>, id: &str) -> Result<(), ServiceError> {
    let Some(info) = info(tx, id)? else {
        return Ok(());
    };
    let (due_at, due_date): (Option<String>, Option<String>) = tx.query_row(
        "SELECT due_at,COALESCE(due_date,planned_date) FROM tasks WHERE id=?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let due = due_at.or(due_date.as_deref().map(date_deadline).transpose()?);
    super::periodic::sync(tx, id, &info.config, due.as_deref(), Utc::now())
}
