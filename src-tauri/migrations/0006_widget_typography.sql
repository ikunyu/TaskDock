ALTER TABLE desktop_settings ADD COLUMN widget_font_size INTEGER NOT NULL DEFAULT 13 CHECK(widget_font_size BETWEEN 10 AND 20);
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
