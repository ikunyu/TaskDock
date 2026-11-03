use super::{model::SearchFilter, query::search_ids};
use crate::{db::SCHEMA_VERSION, error::ServiceError, tasks::validation};
use rusqlite::{types::ValueRef, TransactionBehavior};
use std::io::Write;

#[derive(Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Json,
    Csv,
}
pub fn extension(format: ExportFormat) -> &'static str {
    match format {
        ExportFormat::Json => "json",
        ExportFormat::Csv => "csv",
    }
}
pub fn write_connection(
    c: &mut rusqlite::Connection,
    out: &mut impl Write,
    format: ExportFormat,
    filter: Option<SearchFilter>,
) -> Result<(), ServiceError> {
    let tx = c.transaction_with_behavior(TransactionBehavior::Deferred)?;
    match format {
        ExportFormat::Json => {
            let header = serde_json::json!({"format":"taskdock-data","formatVersion":1,"appVersion":env!("CARGO_PKG_VERSION"),"schemaVersion":SCHEMA_VERSION,"exportedAt":validation::now(),"compatibility":{"import":"reserved; no import IPC in 1.0.0","unknownFormatVersion":"reject","futureSchemaVersion":"reject"}});
            let mut bytes = serde_json::to_vec(&header)?;
            bytes.pop();
            out.write_all(&bytes)?;
            out.write_all(b",\"tables\":{")?;
            for (i, table) in [
                "schema_migrations",
                "app_settings",
                "desktop_settings",
                "window_state",
                "data_settings",
                "backup_records",
                "task_state",
                "categories",
                "tasks",
                "subtasks",
                "task_schedule",
                "recurrence_series",
                "reminders",
                "reminder_delivery_guard",
                "task_history",
            ]
            .into_iter()
            .enumerate()
            {
                if i > 0 {
                    out.write_all(b",")?;
                }
                serde_json::to_writer(&mut *out, table)?;
                out.write_all(b":[")?;
                // Table names come only from this fixed whitelist, never from IPC.
                let mut q = tx.prepare(&format!("SELECT * FROM {table}"))?;
                let names = q.column_names().map(|s| s.to_string()).collect::<Vec<_>>();
                let mut rows = q.query([])?;
                let mut first = true;
                while let Some(row) = rows.next()? {
                    if !first {
                        out.write_all(b",")?;
                    }
                    first = false;
                    let mut object = serde_json::Map::new();
                    for (col, name) in names.iter().enumerate() {
                        let v = match row.get_ref(col)? {
                            ValueRef::Null => serde_json::Value::Null,
                            ValueRef::Integer(n) => n.into(),
                            ValueRef::Real(n) => serde_json::json!(n),
                            ValueRef::Text(t) => String::from_utf8_lossy(t).to_string().into(),
                            ValueRef::Blob(_) => {
                                return Err(ServiceError::Validation(
                                    "数据库含不支持的二进制字段，导出已中止。",
                                ))
                            }
                        };
                        object.insert(name.clone(), v);
                    }
                    serde_json::to_writer(&mut *out, &object)?;
                }
                out.write_all(b"]")?;
            }
            out.write_all(b"}}\n")?;
        }
        ExportFormat::Csv => {
            out.write_all(b"\xef\xbb\xbf")?;
            csv_row(
                out,
                &[
                    "标题",
                    "状态",
                    "优先级",
                    "分类",
                    "计划日期",
                    "截止日期",
                    "截止时间",
                    "完成日期",
                    "描述",
                    "ID",
                    "删除时间",
                ]
                .map(str::to_owned),
            )?;
            let mut offset = 0;
            loop {
                let ids = if let Some(f) = &filter {
                    search_ids(&tx, f, 500, offset)?
                } else {
                    let mut q = tx.prepare(
                        "SELECT id FROM tasks ORDER BY created_at,id LIMIT 500 OFFSET ?1",
                    )?;
                    let rows = q
                        .query_map([offset], |r| r.get::<_, String>(0))?
                        .collect::<Result<Vec<_>, _>>()?;
                    rows
                };
                if ids.is_empty() {
                    break;
                }
                for t in crate::tasks::repository::read_selected(&tx, &ids)? {
                    let category = if let Some(id) = &t.category_id {
                        tx.query_row("SELECT name FROM categories WHERE id=?1", [id], |r| {
                            r.get::<_, String>(0)
                        })?
                    } else {
                        String::new()
                    };
                    csv_row(
                        out,
                        &[
                            t.title,
                            if t.status == crate::tasks::TaskStatus::Completed {
                                "已完成".into()
                            } else {
                                "未完成".into()
                            },
                            match t.priority.as_str() {
                                "high" => "高",
                                "medium" => "中",
                                "low" => "低",
                                _ => "无",
                            }
                            .into(),
                            category,
                            t.planned_date.unwrap_or_default(),
                            t.due_date.unwrap_or_default(),
                            t.due_at.unwrap_or_default(),
                            t.completed_at.unwrap_or_default(),
                            t.description,
                            t.id,
                            t.deleted_at.unwrap_or_default(),
                        ],
                    )?;
                }
                offset += ids.len() as u32;
            }
        }
    }
    out.flush()?;
    tx.commit()?;
    Ok(())
}
fn csv_row(out: &mut impl Write, fields: &[String]) -> Result<(), ServiceError> {
    for (i, value) in fields.iter().enumerate() {
        if i > 0 {
            out.write_all(b",")?;
        }
        // Quote every cell, double embedded quotes, and prefix dangerous cells with
        // an apostrophe even when a formula marker follows whitespace/control chars.
        let lead = value
            .trim_start_matches(|c: char| c.is_whitespace() || c.is_control() || c == '\u{feff}');
        let dangerous =
            lead.starts_with(['=', '+', '-', '@']) || value.starts_with(['\t', '\r', '\n']);
        let safe = if dangerous {
            format!("'{value}")
        } else {
            value.clone()
        };
        write!(out, "\"{}\"", safe.replace('"', "\"\""))?;
    }
    out.write_all(b"\r\n")?;
    Ok(())
}
