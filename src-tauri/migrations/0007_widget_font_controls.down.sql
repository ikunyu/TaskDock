-- Developer-only explicit transaction. Schema7 backup records prevent rollback.
CREATE TABLE backup_records_v6 (
 id TEXT PRIMARY KEY NOT NULL,
 filename TEXT NOT NULL UNIQUE,
 kind TEXT NOT NULL CHECK(kind IN('auto','manual','safety')),
 created_at TEXT NOT NULL,
 sha256 TEXT NOT NULL CHECK(length(sha256)=64),
 bytes INTEGER NOT NULL CHECK(bytes>=0),
 schema_version INTEGER NOT NULL CHECK(schema_version BETWEEN 1 AND 6)
) STRICT;
INSERT INTO backup_records_v6 SELECT * FROM backup_records;
DROP TABLE backup_records;
ALTER TABLE backup_records_v6 RENAME TO backup_records;
ALTER TABLE desktop_settings DROP COLUMN widget_week_date_font_size;
ALTER TABLE desktop_settings DROP COLUMN widget_week_font_size;
ALTER TABLE desktop_settings DROP COLUMN widget_date_font_size;
ALTER TABLE desktop_settings DROP COLUMN widget_compact_font_size;
DELETE FROM schema_migrations WHERE version=7;
