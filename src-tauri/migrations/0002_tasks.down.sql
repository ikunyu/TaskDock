-- Run only inside an explicit transaction, after confirming the database is empty.
-- A nonempty business database fails this guard instead of losing task data.
CREATE TEMP TABLE taskdock_rollback_guard (n INTEGER CHECK (n = 0));
INSERT INTO taskdock_rollback_guard SELECT (SELECT COUNT(*) FROM tasks) + (SELECT COUNT(*) FROM categories) + (SELECT COUNT(*) FROM task_history);
DROP TABLE taskdock_rollback_guard;
DROP TABLE task_history;
DROP TABLE subtasks;
DROP TABLE tasks;
DROP TABLE categories;
DROP TABLE task_state;
DELETE FROM schema_migrations WHERE version = 2;
