-- Developer-only explicit transaction; never discard backup/delivery data.
CREATE TEMP TABLE taskdock_data_rollback_guard(n INTEGER CHECK(n=0));
INSERT INTO taskdock_data_rollback_guard SELECT
 (SELECT COUNT(*) FROM backup_records)+(SELECT COUNT(*) FROM reminder_delivery_guard);
DROP TABLE taskdock_data_rollback_guard;
DROP INDEX idx_history_time;
DROP INDEX idx_tasks_search_plan;
DROP INDEX idx_tasks_search_due_date;
DROP INDEX idx_tasks_search_due_at;
DROP INDEX idx_tasks_search_status;
DROP INDEX idx_tasks_search_recent;
DROP TABLE reminder_delivery_guard;
DROP TABLE backup_records;
DROP TABLE data_settings;
DELETE FROM schema_migrations WHERE version=5;
