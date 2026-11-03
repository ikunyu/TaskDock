-- Developer-only: run in an explicit transaction and roll back on any failure.
-- Refuse to discard a lock or an opacity value unsupported by Schema 7.
CREATE TEMP TABLE taskdock_v8_rollback_guard(value INTEGER CHECK(value=0));
INSERT INTO taskdock_v8_rollback_guard
SELECT COUNT(*) FROM desktop_settings WHERE widget_locked<>0 OR widget_opacity<60;
DROP TABLE taskdock_v8_rollback_guard;

CREATE TABLE desktop_settings_v7 (
    id INTEGER PRIMARY KEY CHECK(id=1),
    widget_opacity INTEGER NOT NULL DEFAULT 100 CHECK(widget_opacity BETWEEN 60 AND 100),
    widget_topmost INTEGER NOT NULL DEFAULT 0 CHECK(widget_topmost IN (0,1)),
    widget_visible INTEGER NOT NULL DEFAULT 0 CHECK(widget_visible IN (0,1)),
    shortcut TEXT NOT NULL DEFAULT 'Ctrl+Alt+T' CHECK(length(shortcut) BETWEEN 3 AND 100),
    autostart INTEGER NOT NULL DEFAULT 0 CHECK(autostart IN (0,1)),
    close_behavior TEXT NOT NULL DEFAULT 'hide' CHECK(close_behavior IN ('hide','exit')),
    widget_font_size INTEGER NOT NULL DEFAULT 13 CHECK(widget_font_size BETWEEN 10 AND 20),
    widget_compact_font_size INTEGER NOT NULL DEFAULT 14 CHECK(widget_compact_font_size BETWEEN 10 AND 24),
    widget_date_font_size INTEGER NOT NULL DEFAULT 10 CHECK(widget_date_font_size BETWEEN 8 AND 18),
    widget_week_font_size INTEGER NOT NULL DEFAULT 15 CHECK(widget_week_font_size BETWEEN 10 AND 26),
    widget_week_date_font_size INTEGER NOT NULL DEFAULT 10 CHECK(widget_week_date_font_size BETWEEN 8 AND 18)
) STRICT;
INSERT INTO desktop_settings_v7 SELECT id,widget_opacity,widget_topmost,widget_visible,shortcut,autostart,close_behavior,
    widget_font_size,widget_compact_font_size,widget_date_font_size,widget_week_font_size,widget_week_date_font_size
FROM desktop_settings;
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
DROP TABLE desktop_settings;
ALTER TABLE desktop_settings_v7 RENAME TO desktop_settings;
DELETE FROM schema_migrations WHERE version=8;
