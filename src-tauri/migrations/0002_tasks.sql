CREATE TABLE task_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    revision INTEGER NOT NULL DEFAULT 0 CHECK (revision BETWEEN 0 AND 9007199254740991)
) STRICT;
INSERT INTO task_state (id, revision) VALUES (1, 0);

CREATE TABLE categories (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36 AND substr(id,9,1) = '-' AND substr(id,14,1) = '-' AND substr(id,19,1) = '-' AND substr(id,24,1) = '-' AND length(replace(id,'-','')) = 32 AND replace(id,'-','') NOT GLOB '*[^0-9a-f]*'),
    name TEXT NOT NULL COLLATE NOCASE UNIQUE CHECK (length(trim(name)) BETWEEN 1 AND 100),
    color TEXT NOT NULL CHECK (length(color) = 7 AND substr(color,1,1) = '#' AND substr(color,2) NOT GLOB '*[^0-9a-fA-F]*'),
    sort_order INTEGER NOT NULL DEFAULT 0,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version BETWEEN 1 AND 9007199254740991),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;
CREATE TABLE tasks (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36 AND substr(id,9,1) = '-' AND substr(id,14,1) = '-' AND substr(id,19,1) = '-' AND substr(id,24,1) = '-' AND length(replace(id,'-','')) = 32 AND replace(id,'-','') NOT GLOB '*[^0-9a-f]*'),
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 500),
    description TEXT NOT NULL DEFAULT '' CHECK (length(description) <= 20000),
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','completed')),
    priority TEXT NOT NULL DEFAULT 'none' CHECK (priority IN ('none','low','medium','high')),
    category_id TEXT REFERENCES categories(id) ON DELETE SET NULL,
    planned_date TEXT CHECK (planned_date IS NULL OR (length(planned_date) = 10 AND planned_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]' AND planned_date BETWEEN '0001-01-01' AND '9999-12-31' AND date(planned_date,'+0 days') IS NOT NULL AND date(planned_date,'+0 days') = planned_date)),
    due_date TEXT CHECK (due_date IS NULL OR (length(due_date) = 10 AND due_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]' AND due_date BETWEEN '0001-01-01' AND '9999-12-31' AND date(due_date,'+0 days') IS NOT NULL AND date(due_date,'+0 days') = due_date)),
    due_at TEXT CHECK (due_at IS NULL OR (length(due_at) = 24 AND due_at GLOB '????-??-??T??:??:??.???Z' AND strftime('%Y-%m-%dT%H:%M:%fZ',due_at,'+0 days') IS NOT NULL AND strftime('%Y-%m-%dT%H:%M:%fZ',due_at,'+0 days') = due_at)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    deleted_at TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version BETWEEN 1 AND 9007199254740991),
    CHECK (due_date IS NULL OR due_at IS NULL),
    CHECK ((status = 'pending' AND completed_at IS NULL) OR (status = 'completed' AND completed_at IS NOT NULL))
) STRICT;
CREATE TABLE subtasks (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36 AND substr(id,9,1) = '-' AND substr(id,14,1) = '-' AND substr(id,19,1) = '-' AND substr(id,24,1) = '-' AND length(replace(id,'-','')) = 32 AND replace(id,'-','') NOT GLOB '*[^0-9a-f]*'),
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 500),
    completed_at TEXT,
    sort_order INTEGER NOT NULL CHECK (sort_order >= 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(task_id, sort_order)
) STRICT;
CREATE TABLE task_history (
    id TEXT PRIMARY KEY NOT NULL,
    transaction_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision > 0),
    entity_type TEXT NOT NULL CHECK (entity_type IN ('task','category')),
    entity_id TEXT NOT NULL,
    task_id TEXT REFERENCES tasks(id) ON DELETE RESTRICT,
    operation TEXT NOT NULL CHECK (operation IN ('create','update','complete','reopen','delete','restore','category_create','category_update','category_delete','category_unlink')),
    before_json TEXT CHECK (before_json IS NULL OR json_valid(before_json)),
    after_json TEXT NOT NULL CHECK (json_valid(after_json)),
    occurred_at TEXT NOT NULL
) STRICT;
CREATE INDEX idx_tasks_active_plan ON tasks(status, planned_date, sort_order) WHERE deleted_at IS NULL;
CREATE INDEX idx_tasks_completed ON tasks(completed_at DESC) WHERE deleted_at IS NULL AND status = 'completed';
CREATE INDEX idx_tasks_category ON tasks(category_id);
CREATE INDEX idx_tasks_deleted ON tasks(deleted_at) WHERE deleted_at IS NOT NULL;
CREATE INDEX idx_subtasks_task ON subtasks(task_id, sort_order);
CREATE INDEX idx_history_task ON task_history(task_id, revision);
CREATE INDEX idx_history_revision ON task_history(revision);
