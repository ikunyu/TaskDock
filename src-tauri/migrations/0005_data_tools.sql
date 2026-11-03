CREATE TABLE data_settings (
 id INTEGER PRIMARY KEY CHECK(id=1),
 auto_backup INTEGER NOT NULL DEFAULT 1 CHECK(auto_backup IN(0,1)),
 interval_hours INTEGER NOT NULL DEFAULT 24 CHECK(interval_hours BETWEEN 1 AND 8760),
 retention INTEGER NOT NULL DEFAULT 30 CHECK(retention BETWEEN 1 AND 365),
 version INTEGER NOT NULL DEFAULT 1 CHECK(version BETWEEN 1 AND 9007199254740991),
 last_success TEXT,
 last_attempt TEXT,
 last_error TEXT
) STRICT;
INSERT INTO data_settings(id) VALUES(1);
CREATE TABLE backup_records (
 id TEXT PRIMARY KEY NOT NULL,
 filename TEXT NOT NULL UNIQUE,
 kind TEXT NOT NULL CHECK(kind IN('auto','manual','safety')),
 created_at TEXT NOT NULL,
 sha256 TEXT NOT NULL CHECK(length(sha256)=64),
 bytes INTEGER NOT NULL CHECK(bytes>=0),
 schema_version INTEGER NOT NULL CHECK(schema_version BETWEEN 1 AND 5)
) STRICT;
CREATE INDEX idx_tasks_search_plan ON tasks(planned_date,id) WHERE deleted_at IS NULL;
CREATE INDEX idx_tasks_search_due_date ON tasks(due_date,id) WHERE deleted_at IS NULL;
CREATE INDEX idx_tasks_search_due_at ON tasks(due_at,id) WHERE deleted_at IS NULL;
CREATE INDEX idx_tasks_search_status ON tasks(status,priority,id) WHERE deleted_at IS NULL;
CREATE INDEX idx_tasks_search_recent ON tasks(updated_at DESC,id) WHERE deleted_at IS NULL;
CREATE INDEX idx_history_time ON task_history(occurred_at DESC,id DESC);
CREATE TABLE reminder_delivery_guard (
 delivery_key TEXT PRIMARY KEY NOT NULL,
 claimed_at TEXT NOT NULL
) STRICT;
INSERT OR IGNORE INTO reminder_delivery_guard(delivery_key,claimed_at)
SELECT json_array(COALESCE(s.series_id,'task:'||r.task_id),COALESCE(s.occurrence_date,''),r.slot,r.scheduled_at),r.claimed_at
FROM reminders r LEFT JOIN task_schedule s ON s.task_id=r.task_id WHERE r.claimed_at IS NOT NULL;
