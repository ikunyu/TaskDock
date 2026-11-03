//! Repeated reminders belong to one task, independently of task recurrence.
//! Only the latest missed occurrence and the next occurrence are materialized.
use super::{calendar::*, model::*};
use crate::{error::ServiceError, tasks::validation};
use chrono::{DateTime, Datelike, Duration, Local, Utc};
use rusqlite::{params, Transaction};

pub fn validate_deadline(config: &ScheduleConfig, due: Option<&str>) -> Result<(), ServiceError> {
    if config.reminder_repeat.is_some() {
        let first = instant(
            config
                .at
                .as_deref()
                .ok_or(ServiceError::Validation("定期提醒需要先设置首次提醒时间。"))?,
        )?;
        if let Some(due) = due {
            if first > instant(due)? {
                return Err(ServiceError::Validation("首次提醒不能晚于任务截止时间。"));
            }
        }
    }
    Ok(())
}

fn occurrence(
    first: DateTime<Utc>,
    repeat: &ReminderRepeat,
    index: i64,
) -> Result<Option<DateTime<Utc>>, ServiceError> {
    let count = index
        .checked_mul(i64::from(repeat.interval))
        .ok_or(ServiceError::Validation("提醒间隔超出范围。"))?;
    let at = match repeat.unit {
        ReminderUnit::Days => {
            let Some(target) = first
                .with_timezone(&Local)
                .naive_local()
                .checked_add_signed(Duration::days(count))
            else {
                return Ok(None);
            };
            if !(1..=9999).contains(&target.year()) {
                return Ok(None);
            }
            // Calendar days preserve the user's local clock through DST changes.
            Some(instant(&shift_time(&stamp(first), count)?)?)
        }
        ReminderUnit::Minutes => first.checked_add_signed(Duration::minutes(count)),
        ReminderUnit::Hours => first.checked_add_signed(Duration::hours(count)),
    };
    Ok(at.filter(|t| (1..=9999).contains(&t.year())))
}

pub fn sync(
    tx: &Transaction<'_>,
    id: &str,
    config: &ScheduleConfig,
    due: Option<&str>,
    now: DateTime<Utc>,
) -> Result<(), ServiceError> {
    let Some(repeat) = &config.reminder_repeat else {
        tx.execute("UPDATE reminders SET state='cancelled' WHERE task_id=?1 AND slot GLOB 'periodic:*' AND state IN('pending','notified')",[id])?;
        return Ok(());
    };
    // Also protects calls made after restoring serialized configuration.
    let mut reminder = config.clone();
    reminder.recurrence = None;
    validate(&reminder, None, due.is_some())?;
    // Saving validates first <= cutoff. Old records may have used an unlimited
    // reminder with only a planning date; cancel that obsolete cycle safely
    // instead of letting one legacy rule block the entire scheduler tick.
    let first = instant(config.at.as_deref().ok_or(ServiceError::InvalidSettings)?)?;
    let deadline = due.map(instant).transpose()?;
    let unit = match repeat.unit {
        ReminderUnit::Minutes => "minutes",
        ReminderUnit::Hours => "hours",
        ReminderUnit::Days => "days",
    };
    let prefix = format!("periodic:{}:{}:{}:", stamp(first), unit, repeat.interval);
    // Changing reminder rules cancels their old inbox entries, but title-only
    // edits preserve delivered IDs/state and never re-send the same occurrence.
    tx.execute("UPDATE reminders SET state='cancelled' WHERE task_id=?1 AND slot GLOB 'periodic:*' AND substr(slot,1,?2)<>?3 AND state IN('pending','notified')",params![id,prefix.len() as i64,prefix])?;
    let mut wanted = vec![];
    if deadline.is_none_or(|end| now <= end) {
        let elapsed = match repeat.unit {
            ReminderUnit::Days => (now.with_timezone(&Local).date_naive()
                - first.with_timezone(&Local).date_naive())
            .num_days(),
            ReminderUnit::Hours => (now - first).num_hours(),
            ReminderUnit::Minutes => (now - first).num_minutes(),
        };
        let mut index = elapsed.max(0) / i64::from(repeat.interval);
        let mut current = occurrence(first, repeat, index)?;
        if index > 0 && current.is_some_and(|t| t > now) {
            index -= 1;
            current = occurrence(first, repeat, index)?;
        }
        if let Some(at) = current.filter(|at| deadline.is_none_or(|end| *at <= end)) {
            wanted.push((format!("{prefix}{index}"), stamp(at)));
            if at <= now {
                // Keep one actionable periodic inbox item per task; the old rows
                // and delivery ledger remain available for audit/restore dedup.
                tx.execute("UPDATE reminders SET state='dismissed' WHERE task_id=?1 AND slot GLOB 'periodic:*' AND state='notified' AND scheduled_at<?2",params![id,stamp(at)])?;
            }
            // Keeping the following occurrence also makes a backwards clock
            // adjustment safe when the first occurrence was already delivered.
            if let Some(next) = occurrence(first, repeat, index + 1)?
                .filter(|at| deadline.is_none_or(|end| *at <= end))
            {
                wanted.push((format!("{prefix}{}", index + 1), stamp(next)));
            }
        }
    }
    let mut q = tx.prepare("SELECT slot FROM reminders WHERE task_id=?1 AND slot GLOB 'periodic:*' AND state='pending'")?;
    let pending = q
        .query_map([id], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for slot in pending {
        if !wanted.iter().any(|(key, _)| key == &slot) {
            tx.execute(
                "UPDATE reminders SET state='cancelled' WHERE task_id=?1 AND slot=?2",
                params![id, slot],
            )?;
        }
    }
    for (slot, at) in wanted {
        tx.execute("INSERT INTO reminders(id,task_id,slot,scheduled_at,state) VALUES(?1,?2,?3,?4,'pending') ON CONFLICT(task_id,slot) DO UPDATE SET state='pending',channel=NULL,claimed_at=NULL,delivered_at=NULL WHERE reminders.state='cancelled'",params![validation::id(),id,slot,at])?;
    }
    Ok(())
}

pub fn advance_due(tx: &Transaction<'_>, now: DateTime<Utc>) -> Result<(), ServiceError> {
    // Only tasks with actually due reminders are read, rather than scanning all
    // task JSON every tick. Repeated outages coalesce into one latest reminder.
    let mut q = tx.prepare("SELECT DISTINCT s.task_id,s.config_json,t.due_at,COALESCE(t.due_date,t.planned_date) FROM task_schedule s JOIN tasks t ON t.id=s.task_id JOIN reminders r ON r.task_id=t.id WHERE t.deleted_at IS NULL AND t.status='pending' AND r.slot GLOB 'periodic:*' AND r.state='pending' AND r.scheduled_at<=?1 ORDER BY r.scheduled_at")?;
    let rows = q
        .query_map([stamp(now)], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (id, json, due_at, due_date) in rows {
        let config: ScheduleConfig = serde_json::from_str(&json)?;
        let due = due_at.or(due_date.as_deref().map(date_deadline).transpose()?);
        sync(tx, &id, &config, due.as_deref(), now)?;
    }
    Ok(())
}
