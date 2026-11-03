CREATE TABLE recurrence_series (
 id TEXT PRIMARY KEY NOT NULL,
 rule_json TEXT NOT NULL CHECK(json_valid(rule_json)),
 template_json TEXT NOT NULL CHECK(json_valid(template_json)),
 exclusions_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(exclusions_json) AND json_type(exclusions_json)='array'),
 start_date TEXT NOT NULL CHECK(date(start_date,'+0 days') IS start_date),
 end_date TEXT CHECK(end_date IS NULL OR date(end_date,'+0 days') IS end_date),
 cursor_date TEXT NOT NULL,
 active INTEGER NOT NULL DEFAULT 1 CHECK(active IN(0,1)),
 version INTEGER NOT NULL DEFAULT 1 CHECK(version>0)
) STRICT;
CREATE TABLE task_schedule (
 task_id TEXT PRIMARY KEY NOT NULL REFERENCES tasks(id) ON DELETE RESTRICT,
 config_json TEXT NOT NULL CHECK(json_valid(config_json)),
 series_id TEXT REFERENCES recurrence_series(id) ON DELETE RESTRICT,
 occurrence_date TEXT,
 UNIQUE(series_id,occurrence_date)
) STRICT;
CREATE TABLE reminders (
 id TEXT PRIMARY KEY NOT NULL,
 task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE RESTRICT,
 slot TEXT NOT NULL,
 scheduled_at TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN('pending','notified','cancelled','dismissed')),
 channel TEXT CHECK(channel IN('system','in_app','unknown')),
 claimed_at TEXT,
 delivered_at TEXT,
 UNIQUE(task_id,slot)
) STRICT;
CREATE INDEX idx_reminders_due ON reminders(state,scheduled_at);
CREATE INDEX idx_reminders_task ON reminders(task_id);
CREATE INDEX idx_series_active ON recurrence_series(active,cursor_date);
