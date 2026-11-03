use super::model::*;
use crate::{
    db::Database,
    error::ServiceError,
    tasks::{repository::read_selected, validation},
};
use chrono::{Datelike, Local, NaiveDate, Utc};
use rusqlite::{params, Connection};

const SEARCH: &str = r#"FROM tasks t WHERE t.deleted_at IS NULL
AND (?1='' OR t.title LIKE ?2 ESCAPE '\' OR t.description LIKE ?2 ESCAPE '\' OR EXISTS(SELECT 1 FROM subtasks s WHERE s.task_id=t.id AND s.title LIKE ?2 ESCAPE '\'))
AND (?3 IS NULL OR t.status=?3) AND (?4 IS NULL OR t.priority=?4)
AND (?5 IS NULL OR (?5='uncategorized' AND t.category_id IS NULL) OR t.category_id=?5)
AND (?6 IS NULL OR t.planned_date>=?6) AND (?7 IS NULL OR t.planned_date<=?7)
AND (?8 IS NULL OR COALESCE(t.due_date,date(t.due_at,'localtime'))>=?8)
AND (?9 IS NULL OR COALESCE(t.due_date,date(t.due_at,'localtime'))<=?9)
AND (?10 IS NULL OR (t.status='completed')=?10)
AND (?11 IS NULL OR COALESCE(t.status='pending' AND (t.planned_date<?12 OR t.due_date<?12 OR t.due_at<?13),0)=?11)
AND (?14 IS NULL OR COALESCE(t.planned_date,t.due_date,date(t.due_at,'localtime'))=?14)"#;
fn like(s: &str) -> String {
    format!(
        "%{}%",
        s.replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    )
}
fn validate_filter(f: &SearchFilter) -> Result<(), ServiceError> {
    if f.keyword.chars().count() > 500 || f.offset > 10_000_000 {
        return Err(ServiceError::Validation("搜索条件过长或页码超限。"));
    }
    if f.status
        .as_deref()
        .is_some_and(|v| !["pending", "completed"].contains(&v))
        || f.priority
            .as_deref()
            .is_some_and(|v| !["none", "low", "medium", "high"].contains(&v))
    {
        return Err(ServiceError::Validation("搜索状态或优先级无效。"));
    }
    if let Some(id) = &f.category_id {
        if id != "uncategorized" {
            validation::validate_id(id)?;
        }
    }
    for d in [
        &f.planned_from,
        &f.planned_to,
        &f.due_from,
        &f.due_to,
        &f.calendar_date,
    ]
    .into_iter()
    .flatten()
    {
        validation::date(d)?;
    }
    if matches!((&f.planned_from,&f.planned_to),(Some(a),Some(b)) if a>b)
        || matches!((&f.due_from,&f.due_to),(Some(a),Some(b)) if a>b)
    {
        return Err(ServiceError::Validation("日期起点不能晚于终点。"));
    }
    Ok(())
}
pub(crate) fn search_ids(
    c: &Connection,
    f: &SearchFilter,
    limit: u32,
    offset: u32,
) -> Result<Vec<String>, ServiceError> {
    validate_filter(f)?;
    let sql = format!("SELECT t.id {SEARCH} ORDER BY t.updated_at DESC,t.id LIMIT ?15 OFFSET ?16");
    let mut q = c.prepare(&sql)?;
    let rows = q
        .query_map(
            params![
                f.keyword,
                like(f.keyword.trim()),
                f.status,
                f.priority,
                f.category_id,
                f.planned_from,
                f.planned_to,
                f.due_from,
                f.due_to,
                f.completed,
                f.overdue,
                Local::now().date_naive().to_string(),
                Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                f.calendar_date,
                limit,
                offset
            ],
            |r| r.get(0),
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
impl Database {
    pub fn search(&self, f: SearchFilter) -> Result<SearchPage, ServiceError> {
        let mut c = self.connection.lock().map_err(|_| ServiceError::Lock)?;
        let tx = c.transaction()?;
        let mut ids = search_ids(&tx, &f, 51, f.offset)?;
        let has_more = ids.len() > 50;
        ids.truncate(50);
        let tasks = read_selected(&tx, &ids)?;
        tx.commit()?;
        Ok(SearchPage {
            tasks,
            has_more,
            offset: f.offset,
        })
    }
    pub fn calendar(
        &self,
        month: &str,
        category: Option<String>,
    ) -> Result<Vec<CalendarDay>, ServiceError> {
        let start = format!("{month}-01");
        validation::date(&start)?;
        let d = NaiveDate::parse_from_str(&start, "%Y-%m-%d")
            .map_err(|_| ServiceError::Validation("月份无效。"))?;
        let last = (28..=31)
            .rev()
            .find_map(|n| NaiveDate::from_ymd_opt(d.year(), d.month(), n))
            .ok_or(ServiceError::Validation("月份无效。"))?
            .to_string();
        if let Some(id) = &category {
            if id != "uncategorized" {
                validation::validate_id(id)?;
            }
        }
        let c = self.connection.lock().map_err(|_| ServiceError::Lock)?;
        let mut q=c.prepare("SELECT COALESCE(planned_date,due_date,date(due_at,'localtime')) AS day,COUNT(*),SUM(status='completed'),SUM(priority='high'),SUM(status='pending' AND (planned_date<?4 OR due_date<?4 OR due_at<?5) IS 1) FROM tasks WHERE deleted_at IS NULL AND day BETWEEN ?1 AND ?2 AND (?3 IS NULL OR (?3='uncategorized' AND category_id IS NULL) OR category_id=?3) GROUP BY day ORDER BY day")?;
        let rows = q
            .query_map(
                params![
                    start,
                    last,
                    category,
                    Local::now().date_naive().to_string(),
                    validation::now()
                ],
                |r| {
                    Ok(CalendarDay {
                        date: r.get(0)?,
                        total: r.get(1)?,
                        completed: r.get(2)?,
                        high: r.get(3)?,
                        overdue: r.get(4)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}
