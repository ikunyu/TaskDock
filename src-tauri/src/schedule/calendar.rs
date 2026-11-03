use super::model::*;
use crate::{error::ServiceError, tasks::validation};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, SecondsFormat, TimeZone, Utc};

pub fn date(s: &str) -> Result<NaiveDate, ServiceError> {
    validation::date(s)?;
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| ServiceError::Validation("日期无效。"))
}
pub fn instant(s: &str) -> Result<DateTime<Utc>, ServiceError> {
    let t = DateTime::parse_from_rfc3339(s)
        .map_err(|_| ServiceError::Validation("提醒时间需要有效时区。"))?
        .with_timezone(&Utc);
    if !(1..=9999).contains(&t.year()) {
        return Err(ServiceError::Validation("提醒时间超出范围。"));
    }
    Ok(t)
}
pub fn stamp(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Millis, true)
}
pub fn validate(
    c: &ScheduleConfig,
    planned: Option<&str>,
    has_due: bool,
) -> Result<(), ServiceError> {
    if let Some(t) = &c.at {
        instant(t)?;
    }
    if let Some(repeat) = &c.reminder_repeat {
        let max = match repeat.unit {
            ReminderUnit::Minutes => 5_256_000,
            ReminderUnit::Hours => 87_600,
            ReminderUnit::Days => 3650,
        };
        if !(1..=max).contains(&repeat.interval) {
            return Err(ServiceError::Validation(
                "定期提醒间隔必须为正整数，且不能超过3650天。",
            ));
        }
        if c.at.is_none() {
            return Err(ServiceError::Validation("定期提醒需要先设置首次提醒时间。"));
        }
    }
    if c.before_minutes.len() > 4
        || c.before_minutes
            .iter()
            .any(|m| ![10, 30, 60, 1440].contains(m))
    {
        return Err(ServiceError::Validation(
            "提前提醒仅支持10分钟、30分钟、1小时、1天。",
        ));
    }
    if (c.at_deadline || !c.before_minutes.is_empty()) && !has_due {
        return Err(ServiceError::Validation(
            "截止提醒需要先设置截止日期或时间。",
        ));
    }
    if let Some(r) = &c.recurrence {
        let start = date(planned.ok_or(ServiceError::Validation("重复任务需要计划日期。"))?)?;
        if !(1..=3650).contains(&r.interval)
            || r.weekdays.len() > 7
            || r.weekdays.iter().any(|d| *d > 6)
            || r.month_day.is_some_and(|d| !(1..=31).contains(&d))
        {
            return Err(ServiceError::Validation("重复间隔、星期或月日期无效。"));
        }
        if r.frequency == Frequency::Weekly && r.weekdays.is_empty() {
            return Err(ServiceError::Validation("每周重复至少选择一天。"));
        }
        if r.frequency == Frequency::Monthly && r.month_day.is_none() {
            return Err(ServiceError::Validation("请选择每月日期。"));
        }
        if let Some(end) = &r.until {
            if date(end)? < start {
                return Err(ServiceError::Validation("重复结束日期不能早于计划日期。"));
            }
        }
    }
    Ok(())
}
pub fn matches(r: &Recurrence, start: NaiveDate, day: NaiveDate) -> bool {
    let delta = (day - start).num_days();
    if delta < 0 || r.until.as_ref().is_some_and(|s| day.to_string() > *s) {
        return false;
    }
    match r.frequency {
        Frequency::Daily | Frequency::Interval => delta % i64::from(r.interval) == 0,
        Frequency::Weekly => {
            let weeks = (delta + i64::from(start.weekday().num_days_from_monday())) / 7;
            weeks % i64::from(r.interval) == 0
                && r.weekdays.contains(&day.weekday().num_days_from_monday())
        }
        Frequency::Monthly => {
            let months =
                (day.year() - start.year()) * 12 + day.month() as i32 - start.month() as i32;
            let last = (28..=31)
                .rev()
                .find(|d| NaiveDate::from_ymd_opt(day.year(), day.month(), *d).is_some())
                .unwrap_or(28);
            months >= 0
                && months % r.interval as i32 == 0
                && day.day() == r.month_day.unwrap_or(start.day()).min(last)
        }
    }
}
// Preserve local wall-clock time; choose the first DST overlap instant, and
// advance a gap to the first valid minute on the same date.
pub fn shift_time(value: &str, days: i64) -> Result<String, ServiceError> {
    let local = instant(value)?.with_timezone(&Local).naive_local();
    let target = local
        .checked_add_signed(Duration::days(days))
        .ok_or(ServiceError::Validation("时间超出范围。"))?;
    let t = (0..=180)
        .filter_map(|m| target.checked_add_signed(Duration::minutes(m)))
        .take_while(|t| t.date() == target.date())
        .find_map(|t| Local.from_local_datetime(&t).earliest())
        .ok_or(ServiceError::Validation(
            "该日期没有可用本地时间，请调整规则。",
        ))?;
    Ok(stamp(t.with_timezone(&Utc)))
}
pub fn date_deadline(s: &str) -> Result<String, ServiceError> {
    let d = date(s)?
        .and_hms_opt(23, 59, 0)
        .ok_or(ServiceError::Validation("截止日期无效。"))?;
    Ok(stamp(
        Local
            .from_local_datetime(&d)
            .single()
            .ok_or(ServiceError::Validation("截止日期本地时间有歧义。"))?
            .with_timezone(&Utc),
    ))
}

/// Legacy explicit deadlines take precedence; otherwise the task's single
/// completion date is its deadline. Dates remain local calendar dates.
pub fn effective_deadline(
    due_at: Option<&str>,
    due_date: Option<&str>,
    completion_date: Option<&str>,
) -> Result<Option<String>, ServiceError> {
    if let Some(at) = due_at {
        return Ok(Some(stamp(instant(at)?)));
    }
    due_date.or(completion_date).map(date_deadline).transpose()
}
pub fn completion_day(
    due_at: Option<&str>,
    due_date: Option<&str>,
    planned_date: Option<&str>,
) -> Result<Option<String>, ServiceError> {
    if let Some(at) = due_at {
        return Ok(Some(
            instant(at)?.with_timezone(&Local).date_naive().to_string(),
        ));
    }
    Ok(due_date.or(planned_date).map(str::to_owned))
}
