use super::TaskDraft;
use crate::error::ServiceError;
use chrono::{DateTime, Datelike, Local, NaiveDate, SecondsFormat, Utc};
use std::collections::HashSet;

pub fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn validate_id(value: &str) -> Result<(), ServiceError> {
    if uuid::Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value) {
        Ok(())
    } else {
        Err(ServiceError::Validation("标识无效，请重新加载任务。"))
    }
}
pub fn text(value: &str, maximum: usize) -> Result<String, ServiceError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > maximum || value.contains('\0') {
        Err(ServiceError::Validation(
            "标题或名称不能为空，且不能超过长度限制。",
        ))
    } else {
        Ok(value.into())
    }
}
pub fn date(value: &str) -> Result<(), ServiceError> {
    if value.len() == 10
        && NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok_and(|date| {
            (1..=9999).contains(&date.year()) && date.format("%Y-%m-%d").to_string() == value
        })
    {
        Ok(())
    } else {
        Err(ServiceError::Validation(
            "日期无效，请使用有效的年、月、日。",
        ))
    }
}
pub fn version(value: i64) -> Result<(), ServiceError> {
    if (1..9_007_199_254_740_991).contains(&value) {
        Ok(())
    } else {
        Err(ServiceError::Validation("数据版本无效，请重新加载。"))
    }
}
/// Apply only to user creation, never to historical edits, restore or generated instances.
pub fn creation_dates(draft: &TaskDraft, today: NaiveDate) -> Result<(), ServiceError> {
    let end = draft
        .schedule
        .as_ref()
        .and_then(|s| s.recurrence.as_ref())
        .and_then(|r| r.until.as_ref());
    for value in [draft.planned_date.as_ref(), draft.due_date.as_ref(), end]
        .into_iter()
        .flatten()
    {
        date(value)?;
        let parsed = NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map_err(|_| ServiceError::Validation("日期无效，请重新选择。"))?;
        if parsed < today {
            return Err(ServiceError::Validation(
                "新建任务的日期只能选择今天或之后的日期。",
            ));
        }
    }
    for value in [
        draft.due_at.as_ref(),
        draft.schedule.as_ref().and_then(|s| s.at.as_ref()),
    ]
    .into_iter()
    .flatten()
    {
        let parsed = DateTime::parse_from_rfc3339(value)
            .map_err(|_| ServiceError::Validation("时间必须包含有效时区。"))?;
        if parsed.with_timezone(&Local).date_naive() < today {
            return Err(ServiceError::Validation(
                "新建任务的日期只能选择今天或之后的日期。",
            ));
        }
    }
    Ok(())
}
pub fn color(value: &str) -> Result<(), ServiceError> {
    if value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|c| c.is_ascii_hexdigit())
    {
        Ok(())
    } else {
        Err(ServiceError::Validation("分类颜色必须为 #RRGGBB。"))
    }
}
pub fn draft(mut draft: TaskDraft) -> Result<TaskDraft, ServiceError> {
    draft.title = text(&draft.title, 500)?;
    if draft.description.chars().count() > 20_000
        || draft.description.contains('\0')
        || draft.subtasks.len() > 200
    {
        return Err(ServiceError::Validation(
            "描述最多 20000 字，子任务最多 200 项。",
        ));
    }
    if !(-9_007_199_254_740_991..=9_007_199_254_740_991).contains(&draft.sort_order) {
        return Err(ServiceError::Validation("排序值超出范围。"));
    }
    if let Some(value) = &draft.category_id {
        validate_id(value)?;
    }
    if let Some(value) = &draft.planned_date {
        date(value)?;
    }
    if let Some(value) = &draft.due_date {
        date(value)?;
    }
    if draft.due_date.is_some() && draft.due_at.is_some() {
        return Err(ServiceError::Validation("截止日期和精确时间只能选择一种。"));
    }
    if let Some(value) = &draft.due_at {
        let parsed = DateTime::parse_from_rfc3339(value)
            .map_err(|_| ServiceError::Validation("截止时间必须包含有效时区。"))?
            .with_timezone(&Utc);
        if !(1..=9999).contains(&parsed.year()) {
            return Err(ServiceError::Validation("截止时间超出范围。"));
        }
        draft.due_at = Some(parsed.to_rfc3339_opts(SecondsFormat::Millis, true));
    }
    let mut ids = HashSet::new();
    for item in &mut draft.subtasks {
        item.title = text(&item.title, 500)?;
        if let Some(value) = &item.id {
            validate_id(value)?;
            if !ids.insert(value.clone()) {
                return Err(ServiceError::Validation("子任务标识不能重复。"));
            }
        }
    }
    Ok(draft)
}
