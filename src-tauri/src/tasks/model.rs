use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    #[default]
    Pending,
    Completed,
}
impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Completed => "completed",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    #[default]
    None,
    Low,
    Medium,
    High,
}
impl Priority {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubtaskDraft {
    pub id: Option<String>,
    pub title: String,
    pub completed: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskDraft {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub priority: Priority,
    pub category_id: Option<String>,
    pub planned_date: Option<String>,
    pub due_date: Option<String>,
    pub due_at: Option<String>,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default)]
    pub subtasks: Vec<SubtaskDraft>,
    pub schedule: Option<crate::schedule::ScheduleConfig>,
    #[serde(default)]
    pub repeat_scope: crate::schedule::EditScope,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subtask {
    pub id: String,
    pub title: String,
    pub completed_at: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub total: usize,
    pub completed: usize,
    pub percent: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: Priority,
    pub category_id: Option<String>,
    pub planned_date: Option<String>,
    pub due_date: Option<String>,
    pub due_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub deleted_at: Option<String>,
    pub sort_order: i64,
    pub version: i64,
    pub subtasks: Vec<Subtask>,
    pub progress: Progress,
    pub schedule: Option<crate::schedule::ScheduleInfo>,
}
impl Task {
    pub fn calculate_progress(&mut self) {
        let total = self.subtasks.len();
        let completed = self
            .subtasks
            .iter()
            .filter(|item| item.completed_at.is_some())
            .count();
        self.progress = Progress {
            total,
            completed,
            percent: (completed * 100).checked_div(total).unwrap_or(0) as u32,
        };
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub name: String,
    pub color: String,
    pub sort_order: i64,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CategoryDraft {
    pub id: Option<String>,
    pub expected_version: Option<i64>,
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub sort_order: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSnapshot {
    pub revision: i64,
    pub tasks: Vec<Task>,
    pub categories: Vec<Category>,
    pub reminders: Vec<crate::schedule::Reminder>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationResult {
    pub snapshot: TaskSnapshot,
    pub task_id: Option<String>,
    pub category_id: Option<String>,
}
