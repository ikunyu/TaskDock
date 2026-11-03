CREATE TABLE desktop_settings (
    id INTEGER PRIMARY KEY CHECK(id=1),
    widget_opacity INTEGER NOT NULL DEFAULT 100 CHECK(widget_opacity BETWEEN 60 AND 100),
    widget_topmost INTEGER NOT NULL DEFAULT 0 CHECK(widget_topmost IN (0,1)),
    widget_visible INTEGER NOT NULL DEFAULT 0 CHECK(widget_visible IN (0,1)),
    shortcut TEXT NOT NULL DEFAULT 'Ctrl+Alt+T' CHECK(length(shortcut) BETWEEN 3 AND 100),
    autostart INTEGER NOT NULL DEFAULT 0 CHECK(autostart IN (0,1)),
    close_behavior TEXT NOT NULL DEFAULT 'hide' CHECK(close_behavior IN ('hide','exit'))
) STRICT;
INSERT INTO desktop_settings(id) VALUES(1);
CREATE TABLE window_state (
    label TEXT PRIMARY KEY CHECK(label IN ('main','widget','quick-entry')),
    x INTEGER NOT NULL CHECK(x BETWEEN -1000000 AND 1000000),
    y INTEGER NOT NULL CHECK(y BETWEEN -1000000 AND 1000000),
    width REAL NOT NULL CHECK(width BETWEEN 100 AND 10000),
    height REAL NOT NULL CHECK(height BETWEEN 100 AND 10000),
    scale REAL NOT NULL CHECK(scale BETWEEN 0.25 AND 8)
) STRICT;
