use crate::error::ServiceError;
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Mutex, time::Duration};

pub const SCHEMA_VERSION: u32 = 8;
struct Migration {
    version: u32,
    name: &'static str,
    sql: &'static str,
}
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial_settings",
        sql: include_str!("../migrations/0001_initial.sql"),
    },
    Migration {
        version: 2,
        name: "task_management",
        sql: include_str!("../migrations/0002_tasks.sql"),
    },
    Migration {
        version: 3,
        name: "desktop_integration",
        sql: include_str!("../migrations/0003_desktop.sql"),
    },
    Migration {
        version: 4,
        name: "scheduling",
        sql: include_str!("../migrations/0004_scheduling.sql"),
    },
    Migration {
        version: 5,
        name: "data_tools",
        sql: include_str!("../migrations/0005_data_tools.sql"),
    },
    Migration {
        version: 6,
        name: "widget_typography",
        sql: include_str!("../migrations/0006_widget_typography.sql"),
    },
    Migration {
        version: 7,
        name: "widget_font_controls",
        sql: include_str!("../migrations/0007_widget_font_controls.sql"),
    },
    Migration {
        version: 8,
        name: "widget_lock",
        sql: include_str!("../migrations/0008_widget_lock.sql"),
    },
];

pub(crate) fn validate_backup_ledger(c: &Connection) -> Result<u32, ServiceError> {
    let mut q = c.prepare("SELECT version,name,sql FROM schema_migrations ORDER BY version")?;
    let rows = q
        .query_map([], |r| {
            Ok((
                r.get::<_, u32>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if rows.is_empty() {
        return Err(ServiceError::Validation("不是有效的TaskDock数据库备份。"));
    }
    if rows.last().is_some_and(|r| r.0 > SCHEMA_VERSION) {
        return Err(ServiceError::Validation(
            "备份来自更高版本TaskDock，请先升级程序。",
        ));
    }
    for (i, (v, name, sql)) in rows.iter().enumerate() {
        if MIGRATIONS
            .get(i)
            .is_none_or(|m| m.version != *v || m.name != name || m.sql != sql)
        {
            return Err(ServiceError::Validation("备份迁移记录不兼容或已修改。"));
        }
    }
    // Reject injected triggers/views and altered tables even if the ledger was copied.
    let mut expected = Connection::open_in_memory()?;
    migrate(&mut expected, &MIGRATIONS[..rows.len()])?;
    fn objects(c: &Connection) -> Result<Vec<(String, String, Option<String>)>, ServiceError> {
        let mut q=c.prepare("SELECT type,name,sql FROM sqlite_master WHERE name NOT GLOB 'sqlite_*' ORDER BY type,name")?;
        let out = q
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(out)
    }
    if objects(c)? != objects(&expected)? {
        return Err(ServiceError::Validation(
            "备份表结构不符合TaskDock版本，拒绝恢复。",
        ));
    }
    Ok(rows.len() as u32)
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Theme {
    fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    fn from_str(value: &str) -> Result<Self, ServiceError> {
        match value {
            "system" => Ok(Self::System),
            "light" => Ok(Self::Light),
            "dark" => Ok(Self::Dark),
            _ => Err(ServiceError::InvalidSettings),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SettingsSnapshot {
    pub theme: Theme,
    pub desktop: crate::desktop::settings::DesktopSettings,
    pub revision: i64,
}

pub struct Database {
    pub(crate) connection: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, ServiceError> {
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        // Validate/migrate before changing journal mode on a potentially newer database.
        migrate(&mut connection, MIGRATIONS)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn settings(&self) -> Result<SettingsSnapshot, ServiceError> {
        let connection = self.connection.lock().map_err(|_| ServiceError::Lock)?;
        read_settings(&connection)
    }

    pub fn set_theme(&self, theme: Theme) -> Result<SettingsSnapshot, ServiceError> {
        let mut connection = self.connection.lock().map_err(|_| ServiceError::Lock)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "UPDATE app_settings SET theme = ?1, revision = revision + 1,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = 1",
            params![theme.as_str()],
        )?;
        let snapshot = read_settings(&transaction)?;
        transaction.commit()?;
        Ok(snapshot)
    }
}

pub(crate) fn read_settings(connection: &Connection) -> Result<SettingsSnapshot, ServiceError> {
    let (theme, revision): (String, i64) = connection.query_row(
        "SELECT theme, revision FROM app_settings WHERE id = 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if !(0..=9_007_199_254_740_991).contains(&revision) {
        return Err(ServiceError::InvalidSettings);
    }
    Ok(SettingsSnapshot {
        theme: Theme::from_str(&theme)?,
        desktop: if connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name='desktop_settings'",
            [],
            |r| r.get::<_, i64>(0),
        )? > 0
        {
            crate::desktop::settings::read_desktop(connection)?
        } else {
            Default::default()
        },
        revision,
    })
}

fn migrate(connection: &mut Connection, migrations: &[Migration]) -> Result<(), ServiceError> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
          version INTEGER PRIMARY KEY,
          name TEXT NOT NULL,
          sql TEXT NOT NULL,
          applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        );",
    )?;
    let applied: Vec<(u32, String, String)> = {
        let mut statement = transaction
            .prepare("SELECT version, name, sql FROM schema_migrations ORDER BY version")?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        rows.collect::<Result<_, _>>()?
    };
    for (index, (version, name, sql)) in applied.iter().enumerate() {
        match migrations.get(index) {
            Some(expected)
                if expected.version == *version && expected.name == name && expected.sql == sql => {
            }
            _ => {
                return Err(ServiceError::Migration(
                    "unknown, missing, or modified migration".into(),
                ))
            }
        }
    }
    for migration in migrations.iter().skip(applied.len()) {
        transaction.execute_batch(migration.sql)?;
        transaction.execute(
            "INSERT INTO schema_migrations (version, name, sql) VALUES (?1, ?2, ?3)",
            params![migration.version, migration.name, migration.sql],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_migration_is_idempotent() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection, MIGRATIONS).unwrap();
        migrate(&mut connection, MIGRATIONS).unwrap();
        assert_eq!(read_settings(&connection).unwrap().theme, Theme::System);
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, i64::from(SCHEMA_VERSION));
    }

    #[test]
    fn failed_migration_rolls_back_ddl_and_ledger() {
        let mut connection = Connection::open_in_memory().unwrap();
        let invalid = [Migration {
            version: 1,
            name: "broken",
            sql: "CREATE TABLE should_rollback(id INTEGER); INVALID SQL;",
        }];
        assert!(migrate(&mut connection, &invalid).is_err());
        let count: i64 = connection.query_row("SELECT COUNT(*) FROM sqlite_master WHERE name IN ('should_rollback', 'schema_migrations')", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn failed_upgrade_preserves_existing_settings_and_migration_ledger() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection, &MIGRATIONS[..1]).unwrap();
        connection
            .execute("UPDATE app_settings SET theme = 'dark', revision = 7", [])
            .unwrap();
        let upgrade = [
            Migration { version: 1, name: MIGRATIONS[0].name, sql: MIGRATIONS[0].sql },
            Migration { version: 2, name: "broken_upgrade", sql: "UPDATE app_settings SET theme = 'light'; CREATE TABLE transient(id); INVALID SQL;" },
        ];
        assert!(migrate(&mut connection, &upgrade).is_err());
        assert_eq!(read_settings(&connection).unwrap().theme, Theme::Dark);
        assert_eq!(read_settings(&connection).unwrap().revision, 7);
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
        let transient: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'transient'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(transient, 0);
    }

    #[test]
    fn rejects_future_or_modified_migration_without_changing_settings() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection, MIGRATIONS).unwrap();
        connection
            .execute("UPDATE schema_migrations SET sql = 'modified'", [])
            .unwrap();
        assert!(matches!(
            migrate(&mut connection, MIGRATIONS),
            Err(ServiceError::Migration(_))
        ));
        assert_eq!(read_settings(&connection).unwrap().revision, 0);
        for migration in MIGRATIONS {
            connection
                .execute(
                    "UPDATE schema_migrations SET sql=?1 WHERE version=?2",
                    params![migration.sql, migration.version],
                )
                .unwrap();
        }
        connection
            .execute(
                "UPDATE schema_migrations SET version = 9 WHERE version=2",
                [],
            )
            .unwrap();
        assert!(matches!(
            migrate(&mut connection, MIGRATIONS),
            Err(ServiceError::Migration(_))
        ));
    }

    #[test]
    fn settings_survive_reopen_and_revision_increases() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.sqlite3");
        {
            let database = Database::open(&path).unwrap();
            assert_eq!(database.set_theme(Theme::Dark).unwrap().revision, 1);
            assert_eq!(database.set_theme(Theme::Light).unwrap().revision, 2);
        }
        let database = Database::open(&path).unwrap();
        assert_eq!(database.settings().unwrap().theme, Theme::Light);
        assert_eq!(database.settings().unwrap().revision, 2);
        let connection = database.connection.lock().unwrap();
        assert_eq!(
            connection
                .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
                .unwrap(),
            "wal"
        );
        assert_eq!(
            connection
                .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn invalid_theme_and_missing_singleton_do_not_mutate_revision() {
        let dir = tempfile::tempdir().unwrap();
        let database = Database::open(&dir.path().join("test.sqlite3")).unwrap();
        {
            let connection = database.connection.lock().unwrap();
            assert!(connection
                .execute("UPDATE app_settings SET theme = 'invalid'", [])
                .is_err());
            assert_eq!(read_settings(&connection).unwrap().revision, 0);
            connection.execute("DELETE FROM app_settings", []).unwrap();
        }
        assert!(database.set_theme(Theme::Dark).is_err());
    }

    #[test]
    fn schema_one_file_upgrades_without_losing_settings_and_is_repeatable() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("old.sqlite3");
        {
            let mut connection = Connection::open(&path).unwrap();
            migrate(&mut connection, &MIGRATIONS[..1]).unwrap();
            connection
                .execute("UPDATE app_settings SET theme='dark',revision=11", [])
                .unwrap();
        }
        for _ in 0..2 {
            let database = Database::open(&path).unwrap();
            assert_eq!(database.settings().unwrap().theme, Theme::Dark);
            assert_eq!(database.settings().unwrap().revision, 11);
            assert!(database.task_snapshot().unwrap().tasks.is_empty());
            let count: i64 = database
                .connection
                .lock()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, i64::from(SCHEMA_VERSION));
        }
    }

    #[test]
    fn actual_schema_two_migration_failure_preserves_existing_objects() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection, &MIGRATIONS[..1]).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE tasks (sentinel TEXT); INSERT INTO tasks VALUES ('preserve');",
            )
            .unwrap();
        assert!(migrate(&mut connection, MIGRATIONS).is_err());
        assert_eq!(
            connection
                .query_row("SELECT sentinel FROM tasks", [], |row| row
                    .get::<_, String>(0))
                .unwrap(),
            "preserve"
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name IN ('task_state','categories')",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }

    #[test]
    fn empty_schema_two_can_roll_back_but_nonempty_database_cannot() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection, &MIGRATIONS[..2]).unwrap();
        {
            let transaction = connection.transaction().unwrap();
            transaction
                .execute_batch(include_str!("../migrations/0002_tasks.down.sql"))
                .unwrap();
            transaction.commit().unwrap();
        }
        assert_eq!(read_settings(&connection).unwrap().theme, Theme::System);
        migrate(&mut connection, &MIGRATIONS[..2]).unwrap();
        let database = Database {
            connection: Mutex::new(connection),
        };
        database
            .create_task(crate::tasks::TaskDraft {
                title: "保留任务".into(),
                ..Default::default()
            })
            .unwrap();
        {
            let mut connection = database.connection.lock().unwrap();
            let transaction = connection.transaction().unwrap();
            assert!(transaction
                .execute_batch(include_str!("../migrations/0002_tasks.down.sql"))
                .is_err());
        }
        assert_eq!(database.task_snapshot().unwrap().tasks.len(), 1);
    }
    #[test]
    fn schema_two_tasks_survive_desktop_upgrade_and_down_migration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old-two.sqlite3");
        let mut c = Connection::open(&path).unwrap();
        c.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrate(&mut c, &MIGRATIONS[..2]).unwrap();
        let db = Database {
            connection: Mutex::new(c),
        };
        db.create_task(crate::tasks::TaskDraft {
            title: "已有任务".into(),
            ..Default::default()
        })
        .unwrap();
        db.set_theme(Theme::Dark).unwrap();
        drop(db);
        for _ in 0..2 {
            let db = Database::open(&path).unwrap();
            assert_eq!(db.task_snapshot().unwrap().tasks[0].title, "已有任务");
            assert_eq!(db.settings().unwrap().theme, Theme::Dark);
            assert!(!db.settings().unwrap().desktop.autostart);
        }
        let mut c = Connection::open(&path).unwrap();
        let tx = c.transaction().unwrap();
        tx.execute_batch(include_str!("../migrations/0003_desktop.down.sql"))
            .unwrap();
        tx.commit().unwrap();
        drop(c);
        let db = Database::open(&path).unwrap();
        assert_eq!(db.task_snapshot().unwrap().tasks.len(), 1);
    }
    #[test]
    fn failed_schema_three_ddl_rolls_back_without_touching_schema_two() {
        let mut c = Connection::open_in_memory().unwrap();
        migrate(&mut c, &MIGRATIONS[..2]).unwrap();
        c.execute_batch(
            "CREATE TABLE window_state(sentinel TEXT); INSERT INTO window_state VALUES('keep');",
        )
        .unwrap();
        assert!(migrate(&mut c, MIGRATIONS).is_err());
        let count: i64 = c
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
        let count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name='desktop_settings'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
        assert_eq!(
            c.query_row("SELECT sentinel FROM window_state", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "keep"
        );
    }
}
