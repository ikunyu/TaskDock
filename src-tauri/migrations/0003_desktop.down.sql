-- Explicit developer transaction only. Task tables and settings theme remain intact.
DROP TABLE window_state;
DROP TABLE desktop_settings;
DELETE FROM schema_migrations WHERE version=3;
