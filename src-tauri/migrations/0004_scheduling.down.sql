-- Developer-only, explicit transaction. Never discard scheduling data.
CREATE TEMP TABLE taskdock_schedule_rollback_guard (n INTEGER CHECK (n=0));
INSERT INTO taskdock_schedule_rollback_guard SELECT
 (SELECT COUNT(*) FROM task_schedule) + (SELECT COUNT(*) FROM reminders) +
 (SELECT COUNT(*) FROM recurrence_series);
DROP TABLE taskdock_schedule_rollback_guard;
DROP TABLE reminders;
DROP TABLE task_schedule;
DROP TABLE recurrence_series;
DELETE FROM schema_migrations WHERE version=4;
