use crate::tasks::Task;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupSettings {
    pub auto_backup: bool,
    pub interval_hours: i64,
    pub retention: i64,
    pub version: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupEntry {
    pub id: String,
    pub filename: String,
    pub kind: String,
    pub created_at: String,
    pub sha256: String,
    pub bytes: u64,
    pub schema_version: u32,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataStatus {
    pub data_directory: String,
    pub settings: BackupSettings,
    pub backups: Vec<BackupEntry>,
    pub last_success: Option<String>,
    pub last_attempt: Option<String>,
    pub last_error: Option<String>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchFilter {
    #[serde(default)]
    pub keyword: String,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub category_id: Option<String>,
    pub planned_from: Option<String>,
    pub planned_to: Option<String>,
    pub due_from: Option<String>,
    pub due_to: Option<String>,
    pub completed: Option<bool>,
    pub overdue: Option<bool>,
    pub calendar_date: Option<String>,
    #[serde(default)]
    pub offset: u32,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPage {
    pub tasks: Vec<Task>,
    pub has_more: bool,
    pub offset: u32,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDay {
    pub date: String,
    pub total: i64,
    pub completed: i64,
    pub high: i64,
    pub overdue: i64,
}
