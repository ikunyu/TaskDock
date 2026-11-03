mod calendar;
mod commands;
mod model;
mod periodic;
pub(crate) mod repository;
mod runtime;
pub use commands::*;
pub use model::*;
pub use runtime::start;

use crate::{
    db::Database,
    error::ServiceError,
    tasks::{validation, MutationResult},
};
use chrono::{Duration, Utc};
use rusqlite::params;

/// Validate serialized scheduling rules before a restored file can replace the
/// live database. SQLite JSON validity alone does not validate recurrence enums
/// or prevent a zero interval from reaching calendar arithmetic.
pub(crate) fn validate_restored(c: &rusqlite::Connection) -> Result<(), ServiceError> {
    let mut q=c.prepare("SELECT rule_json,template_json,start_date,cursor_date,exclusions_json FROM recurrence_series")?;
    let rows = q.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
        ))
    })?;
    for row in rows {
        let (rule, template, start, cursor, excluded) = row?;
        let rule: Recurrence = serde_json::from_str(&rule)?;
        let template: crate::tasks::TaskDraft = serde_json::from_str(&template)?;
        let template = validation::draft(template)?;
        validation::date(&start)?;
        validation::date(&cursor)?;
        if cursor < start {
            return Err(ServiceError::Validation("备份中的重复游标无效。"));
        }
        let dates: Vec<String> = serde_json::from_str(&excluded)?;
        for d in dates {
            validation::date(&d)?;
        }
        let mut config = template.schedule.unwrap_or_default();
        config.recurrence = Some(rule);
        calendar::validate(
            &config,
            Some(&start),
            template.due_date.is_some()
                || template.due_at.is_some()
                || template.planned_date.is_some(),
        )?;
        let due = template.due_at.or(template
            .due_date
            .as_deref()
            .map(calendar::date_deadline)
            .transpose()?);
        periodic::validate_deadline(&config, due.as_deref())?;
    }
    let mut q=c.prepare("SELECT s.config_json,COALESCE(r.start_date,t.planned_date),t.due_at,t.due_date FROM task_schedule s JOIN tasks t ON t.id=s.task_id LEFT JOIN recurrence_series r ON r.id=s.series_id")?;
    for row in q.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<String>>(3)?,
        ))
    })? {
        let (config, planned, due_at, due_date) = row?;
        let config: ScheduleConfig = serde_json::from_str(&config)?;
        let due = due_at.or(due_date
            .as_deref()
            .map(calendar::date_deadline)
            .transpose()?);
        calendar::validate(
            &config,
            planned.as_deref(),
            due.is_some() || planned.is_some(),
        )?;
        periodic::validate_deadline(&config, due.as_deref())?;
    }
    Ok(())
}

impl Database {
    /// Check cancellation immediately before enqueueing to the OS. The existing
    /// database mutex serializes this short operation with task edits/completion.
    pub fn dispatch_reminders(
        &self,
        batch: &[Reminder],
        send: impl FnOnce(Vec<Reminder>),
    ) -> Result<(), ServiceError> {
        let c = self.connection.lock().map_err(|_| ServiceError::Lock)?;
        let mut active = vec![];
        for r in batch {
            let valid:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM reminders r JOIN tasks t ON t.id=r.task_id WHERE r.id=?1 AND r.scheduled_at=?2 AND r.state='notified' AND t.status='pending' AND t.deleted_at IS NULL)",params![r.id,r.scheduled_at],|row|row.get(0))?;
            if valid {
                active.push(r.clone());
            }
        }
        send(active);
        Ok(())
    }
    pub fn schedule_tick(&self) -> Result<Option<(MutationResult, Vec<Reminder>)>, ServiceError> {
        {
            let c = self.connection.lock().map_err(|_| ServiceError::Lock)?;
            let needed: bool = c.query_row(
                "SELECT EXISTS(SELECT 1 FROM reminders WHERE state='pending' AND scheduled_at<=?1)",
                [calendar::stamp(Utc::now())],
                |r| r.get(0),
            )?;
            let needed = needed || repository::generation_needed(&c)?;
            if !needed {
                return Ok(None);
            }
        }
        let mut batch = vec![];
        let result = self.mutate_tasks(|tx, _| {
            repository::generate(tx)?;
            batch = repository::claim(tx)?;
            for r in &batch {
                super::tasks::repository::reminder_audit(tx, &r.task_id, "reminder_claim")?;
            }
            Ok((None, None))
        })?;
        Ok(Some((result, batch)))
    }
    pub fn reminder_action(
        &self,
        id: &str,
        minutes: Option<i64>,
    ) -> Result<MutationResult, ServiceError> {
        validation::validate_id(id)?;
        if minutes.is_some_and(|m| ![10, 30, 60].contains(&m)) {
            return Err(ServiceError::Validation(
                "稍后提醒仅支持10分钟、30分钟、1小时。",
            ));
        }
        self.mutate_tasks(|tx,_|{
            let (task_id,state):(String,String)=tx.query_row("SELECT task_id,state FROM reminders WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|if matches!(e,rusqlite::Error::QueryReturnedNoRows){ServiceError::NotFound}else{e.into()})?;
            let active:bool=tx.query_row("SELECT status='pending' AND deleted_at IS NULL FROM tasks WHERE id=?1",[&task_id],|r|r.get(0))?;
            if !active || state!="notified" {return Err(ServiceError::Validation("该提醒已处理或任务已结束，请刷新。"));}
            tx.execute("UPDATE reminders SET state='dismissed' WHERE id=?1",[id])?;
            if let Some(m)=minutes {
                let at=calendar::stamp(Utc::now()+Duration::minutes(m));
                tx.execute("INSERT INTO reminders(id,task_id,slot,scheduled_at,state) VALUES(?1,?2,'snooze',?3,'pending') ON CONFLICT(task_id,slot) DO UPDATE SET scheduled_at=excluded.scheduled_at,state='pending',channel=NULL,claimed_at=NULL,delivered_at=NULL",params![validation::id(),task_id,at])?;
            }
            super::tasks::repository::reminder_audit(tx,&task_id,if minutes.is_some(){"reminder_snooze"}else{"reminder_dismiss"})?;
            Ok((Some(task_id),None))
        })
    }
}
