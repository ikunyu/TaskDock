//! Portable persistence beside the executable. Never fall back to AppData.
//! Old data is read only; a fully staged snapshot is published on the same volume.
use crate::{data::backup, db::Database, error::ServiceError, tasks::validation};
use serde_json::json;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

pub struct Paths {
    pub data: PathBuf,
    pub logs: PathBuf,
    pub webview: PathBuf,
}
impl Paths {
    pub fn current() -> std::io::Result<Self> {
        let exe = std::env::current_exe()?;
        let root = exe
            .parent()
            .ok_or_else(|| std::io::Error::other("executable directory unavailable"))?;
        Ok(Self {
            data: root.join("data"),
            logs: root.join("logs"),
            webview: root.join("webview"),
        })
    }
}

fn ordinary(path: &Path) -> Result<fs::Metadata, ServiceError> {
    let metadata = fs::symlink_metadata(path)?;
    #[cfg(windows)]
    let reparse = {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let reparse = false;
    if metadata.file_type().is_symlink() || reparse {
        return Err(ServiceError::Validation(
            "数据目录或文件是链接，已停止操作并保留原数据。",
        ));
    }
    Ok(metadata)
}

pub fn writable_directory(path: &Path) -> Result<(), ServiceError> {
    fs::create_dir_all(path)?;
    if !ordinary(path)?.is_dir() {
        return Err(ServiceError::Validation("数据路径不是文件夹。"));
    }
    let probe = path.join(format!(".taskdock-write-{}", validation::id()));
    let mut f = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe)?;
    let result = f.write_all(b"TaskDock").and_then(|_| f.sync_all());
    drop(f);
    // Only this newly-created probe is removed, never a pre-existing user file.
    let cleanup = fs::remove_file(&probe);
    result?;
    cleanup?;
    Ok(())
}

fn copy_file(source: &Path, target: &Path) -> Result<(), ServiceError> {
    if !ordinary(source)?.is_file() {
        return Err(ServiceError::Validation(
            "旧备份中有非普通文件，迁移已停止，原数据保留。",
        ));
    }
    let mut from = fs::File::open(source)?;
    let mut to = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(target)?;
    std::io::copy(&mut from, &mut to)?;
    to.sync_all()?;
    drop(to);
    if backup::hash(source)? != backup::hash(target)? {
        return Err(ServiceError::Validation(
            "备份复制校验失败，原数据和暂存文件已保留。",
        ));
    }
    Ok(())
}

fn finish_snapshot(path: &Path) -> Result<(), ServiceError> {
    // Migrate and validate the staged copy only. No old database is upgraded.
    let db = Database::open(path)?;
    db.settings()?;
    db.task_snapshot()?;
    db.backup_settings()?;
    let c = db.connection.into_inner().map_err(|_| ServiceError::Lock)?;
    crate::schedule::validate_restored(&c)?;
    let busy: i64 = c.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get(0))?;
    if busy != 0 {
        return Err(ServiceError::Validation("迁移暂存库仍被占用，原数据保留。"));
    }
    let mode: String = c.query_row("PRAGMA journal_mode=DELETE", [], |r| r.get(0))?;
    if mode != "delete" {
        return Err(ServiceError::Validation("无法完成迁移暂存库，原数据保留。"));
    }
    backup::validate(&c)?;
    c.close().map_err(|(_, e)| ServiceError::from(e))?;
    OpenOptions::new().write(true).open(path)?.sync_all()?;
    Ok(())
}

pub fn prepare(database: &Path, legacy: &Path) -> Result<(), ServiceError> {
    let destination = database
        .parent()
        .ok_or(ServiceError::Validation("软件数据路径无效。"))?;
    if destination.try_exists()? {
        if !ordinary(destination)?.is_dir() {
            return Err(ServiceError::Validation("软件data路径不是文件夹。"));
        }
        if database.try_exists()? {
            if !ordinary(database)?.is_file() {
                return Err(ServiceError::Validation("软件数据库不是普通文件。"));
            }
            return writable_directory(destination);
        }
        if fs::read_dir(destination)?.next().is_some() {
            return Err(ServiceError::Validation("data文件夹非空但缺少主数据库，已停止初始化；请保留文件并检查数据，不会创建空库覆盖。"));
        }
    }
    let source = legacy.join("taskdock.sqlite3");
    if legacy.try_exists()? && !ordinary(legacy)?.is_dir() {
        return Err(ServiceError::Validation("旧数据路径无效，已停止迁移。"));
    }
    if legacy.join("restore-journal.json").try_exists()? {
        return Err(ServiceError::Validation(
            "旧数据有未完成的恢复操作，请先用原版启动完成恢复，再迁移；原文件已保留。",
        ));
    }
    if !source.try_exists()? {
        let parent = destination
            .parent()
            .ok_or(ServiceError::Validation("软件目录无效。"))?;
        for entry in fs::read_dir(parent)? {
            if entry?
                .file_name()
                .to_string_lossy()
                .starts_with(".taskdock-import-")
            {
                return Err(ServiceError::Validation("发现未完成的迁入暂存文件且旧主库缺失，已停止创建空库，请保留暂存目录并检查数据。"));
            }
        }
        let backups = legacy.join("backups");
        if legacy.join("taskdock.sqlite3-wal").try_exists()?
            || legacy.join("taskdock.sqlite3-shm").try_exists()?
            || (backups.try_exists()?
                && (!ordinary(&backups)?.is_dir() || fs::read_dir(&backups)?.next().is_some()))
        {
            return Err(ServiceError::Validation(
                "旧目录有备份或数据库附属文件但缺少主库，已停止创建空库，请保留原文件。",
            ));
        }
        return writable_directory(destination);
    }
    if !ordinary(&source)?.is_file() {
        return Err(ServiceError::Validation(
            "旧数据库不是普通文件，原文件已保留。",
        ));
    }
    let parent = destination
        .parent()
        .ok_or(ServiceError::Validation("软件目录无效。"))?;
    let staging = parent.join(format!(".taskdock-import-{}", validation::id()));
    fs::create_dir(&staging)?;
    // Intentionally keep a failed staging directory for recovery, rather than
    // recursively removing files which might be the only surviving snapshot.
    let snapshot = staging.join("taskdock.sqlite3");
    {
        let connection = backup::readonly(&source)?;
        backup::validate(&connection)?;
        backup::online(&connection, &snapshot)?;
    }
    finish_snapshot(&snapshot)?;
    let backups = legacy.join("backups");
    if backups.try_exists()? {
        if !ordinary(&backups)?.is_dir() {
            return Err(ServiceError::Validation("旧备份路径无效，原文件保留。"));
        }
        let target = staging.join("backups");
        fs::create_dir(&target)?;
        for entry in fs::read_dir(&backups)? {
            let entry = entry?;
            copy_file(&entry.path(), &target.join(entry.file_name()))?;
        }
    }
    let marker = json!({"format":"taskdock-portable-migration","version":1,"source":legacy,"at":validation::now(),"sha256":backup::hash(&snapshot)?});
    let mut f = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(staging.join("migration.json"))?;
    f.write_all(&serde_json::to_vec_pretty(&marker)?)?;
    f.sync_all()?;
    drop(f);
    if destination.try_exists()? {
        // Removes only the known, empty destination placeholder. A concurrent
        // new file makes remove_dir fail; no recursive overwrite is performed.
        fs::remove_dir(destination)?;
    }
    fs::rename(&staging, destination)?;
    Ok(())
}

pub fn startup_error() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        let title: Vec<u16> = "TaskDock\0".encode_utf16().collect();
        let text: Vec<u16> = "TaskDock 启动失败。请确认软件目录可写，建议将软件放到 D、F 等非系统盘目录，并查看旁边 logs 文件夹。原任务数据不会被删除。\0".encode_utf16().collect();
        // Startup diagnostics only, displayed when the user launches the app.
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONERROR,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db::Theme, tasks::TaskDraft};

    #[test]
    fn migration_preserves_live_wal_source_and_backups_then_prefers_destination() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join("old");
        fs::create_dir(&legacy).unwrap();
        let source = legacy.join("taskdock.sqlite3");
        let original = Database::open(&source).unwrap();
        original.set_theme(Theme::Dark).unwrap();
        original
            .create_task(TaskDraft {
                title: "原有任务".into(),
                ..Default::default()
            })
            .unwrap();
        let backups = legacy.join("backups");
        fs::create_dir(&backups).unwrap();
        let entry = backup::create(&original, &source, &backups, "manual").unwrap();
        let destination = temp.path().join("data/taskdock.sqlite3");
        prepare(&destination, &legacy).unwrap();
        let loaded = Database::open(&destination).unwrap();
        assert_eq!(loaded.settings().unwrap().theme, Theme::Dark);
        assert_eq!(loaded.task_snapshot().unwrap().tasks[0].title, "原有任务");
        assert_eq!(
            backup::hash(&temp.path().join("data/backups").join(&entry.filename)).unwrap(),
            entry.sha256
        );
        assert!(source.exists());
        original
            .create_task(TaskDraft {
                title: "仅原库增加".into(),
                ..Default::default()
            })
            .unwrap();
        prepare(&destination, &legacy).unwrap();
        assert_eq!(loaded.task_snapshot().unwrap().tasks.len(), 1);
        assert_eq!(original.task_snapshot().unwrap().tasks.len(), 2);
    }

    #[test]
    fn corrupt_source_is_preserved_and_never_replaced_by_a_fresh_database() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join("old");
        fs::create_dir(&legacy).unwrap();
        let source = legacy.join("taskdock.sqlite3");
        fs::write(&source, b"corrupt source - preserve").unwrap();
        let destination = temp.path().join("data/taskdock.sqlite3");
        assert!(prepare(&destination, &legacy).is_err());
        assert_eq!(fs::read(&source).unwrap(), b"corrupt source - preserve");
        assert!(!destination.exists());
    }

    #[test]
    fn nonempty_incomplete_destination_and_pending_restore_are_not_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("data");
        fs::create_dir(&data).unwrap();
        fs::write(data.join("important.json"), b"keep").unwrap();
        let legacy = temp.path().join("old");
        let destination = data.join("taskdock.sqlite3");
        assert!(prepare(&destination, &legacy).is_err());
        assert_eq!(fs::read(data.join("important.json")).unwrap(), b"keep");
        assert!(!destination.exists());
        fs::create_dir(&legacy).unwrap();
        fs::write(legacy.join("restore-journal.json"), b"keep journal").unwrap();
        assert!(prepare(&temp.path().join("new/taskdock.sqlite3"), &legacy).is_err());
        assert_eq!(
            fs::read(legacy.join("restore-journal.json")).unwrap(),
            b"keep journal"
        );
    }
}
