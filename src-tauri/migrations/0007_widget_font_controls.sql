ALTER TABLE desktop_settings ADD COLUMN widget_compact_font_size INTEGER NOT NULL DEFAULT 14 CHECK(widget_compact_font_size BETWEEN 10 AND 24);
ALTER TABLE desktop_settings ADD COLUMN widget_date_font_size INTEGER NOT NULL DEFAULT 10 CHECK(widget_date_font_size BETWEEN 8 AND 18);
ALTER TABLE desktop_settings ADD COLUMN widget_week_font_size INTEGER NOT NULL DEFAULT 15 CHECK(widget_week_font_size BETWEEN 10 AND 26);
ALTER TABLE desktop_settings ADD COLUMN widget_week_date_font_size INTEGER NOT NULL DEFAULT 10 CHECK(widget_week_date_font_size BETWEEN 8 AND 18);
UPDATE desktop_settings SET widget_compact_font_size=MAX(14,widget_font_size),widget_date_font_size=MAX(9,widget_font_size-3),widget_week_font_size=widget_font_size+2,widget_week_date_font_size=MAX(9,widget_font_size-3);
CREATE TABLE backup_records_v7 (
 id TEXT PRIMARY KEY NOT NULL,
 filename TEXT NOT NULL UNIQUE,
 kind TEXT NOT NULL CHECK(kind IN('auto','manual','safety')),
 created_at TEXT NOT NULL,
 sha256 TEXT NOT NULL CHECK(length(sha256)=64),
 bytes INTEGER NOT NULL CHECK(bytes>=0),
 schema_version INTEGER NOT NULL CHECK(schema_version BETWEEN 1 AND 7)
) STRICT;
INSERT INTO backup_records_v7 SELECT * FROM backup_records;
DROP TABLE backup_records;
ALTER TABLE backup_records_v7 RENAME TO backup_records;
