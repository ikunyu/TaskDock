use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleConfig {
    pub at: Option<String>,
    #[serde(default)]
    pub reminder_repeat: Option<ReminderRepeat>,
    #[serde(default)]
    pub before_minutes: Vec<i64>,
    #[serde(default)]
    pub at_deadline: bool,
    pub recurrence: Option<Recurrence>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Recurrence {
    pub frequency: Frequency,
    pub interval: u32,
    #[serde(default)]
    pub weekdays: Vec<u32>,
    pub month_day: Option<u32>,
    pub until: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Interval,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EditScope {
    #[default]
    Once,
    Following,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleInfo {
    pub config: ScheduleConfig,
    pub series_id: Option<String>,
    pub occurrence_date: Option<String>,
    pub next_at: Option<String>,
    pub has_notified: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: String,
    pub task_id: String,
    pub title: String,
    pub scheduled_at: String,
    pub state: String,
    pub channel: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReminderRepeat {
    pub interval: u32,
    pub unit: ReminderUnit,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReminderUnit {
    Minutes,
    Hours,
    Days,
}
