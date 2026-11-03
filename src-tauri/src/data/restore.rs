use super::backup;
use crate::{db::Database, error::ServiceError, service::LocalService};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{atomic::Ordering, Arc, Mutex},
};

#[derive(Serialize, Deserialize)]
struct Journal {
    id: String,
    staging: String,
    rollback: String,
    original_hash: String,
    safety: String,
}
fn parent(path: &Path) -> Result<&Path, ServiceError> {
    path.parent()
        .ok_or(ServiceError::Validation("数据库目录无效。"))
}
fn safe_child(root: &Path, name: &str) -> Result<PathBuf, ServiceError> {
    if name.is_empty() || name.contains(['/', '\\', ':']) || name == ".." {
        return Err(ServiceError::Validation("恢复日志路径无效，原文件已保留。"));
    }
    Ok(root.join(name))
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), ServiceError> {
    let mut f = OpenOptions::new().write(true).create_new(true).open(path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    Ok(())
}
fn marker(root: &Path, id: &str) -> PathBuf {
    root.join(format!(".restore-committed-{id}"))
}
fn publish_record(target: &Path, temp: &Path, bytes: &[u8]) -> Result<(), ServiceError> {
    // Never expose a partially-written journal/commit marker on disk-full or
    // process interruption. Both names are on the database volume.
    write_new(temp, bytes)?;
    fs::rename(temp, target)?;
    Ok(())
}

/// Called before opening the sole service connection. An uncommitted replacement
/// is rolled back, including after process/power loss between filesystem steps.
pub(crate) fn recover_interrupted(path: &Path) -> Result<(), ServiceError> {
    let root = parent(path)?;
    let journal = root.join("restore-journal.json");
    if !journal.exists() {
        return Ok(());
    }
    let bytes = fs::read(&journal)?;
    if bytes.len() > 65536 {
        return Err(ServiceError::Validation(
            "恢复日志异常，请保留数据库及回退文件。",
        ));
    }
    let j: Journal = serde_json::from_slice(&bytes)?;
    crate::tasks::validation::validate_id(&j.id)?;
    let commit = marker(root, &j.id);
    if !fs::read(&commit).is_ok_and(|b| b == b"committed") {
        let rollback = safe_child(root, &j.rollback)?;
        if rollback.exists() && backup::hash(&rollback)? == j.original_hash {
            remove_idle_sidecars(path)?;
            if path.exists() {
                fs::rename(
                    path,
                    root.join(format!(
                        ".restore-quarantine-{}.db",
                        crate::tasks::validation::id()
                    )),
                )?;
            }
            fs::rename(rollback, path)?;
        } else if !path.exists() || backup::hash(path)? != j.original_hash {
            let backup_dir = backup::directory(path)?;
            let entry = backup::list(&backup_dir)?
                .into_iter()
                .find(|e| e.filename == j.safety)
                .ok_or(ServiceError::Validation(
                    "恢复安全备份记录缺失，请保留回退文件。",
                ))?;
            let (_, safety) = backup::select(&backup_dir, &entry.id)?;
            backup::validate(&backup::readonly(&safety)?)?;
            let rescue = root.join(format!(".rescue-{}.db", crate::tasks::validation::id()));
            backup::online(&backup::readonly(&safety)?, &rescue)?;
            remove_idle_sidecars(path)?;
            if path.exists() {
                fs::rename(
                    path,
                    root.join(format!(
                        ".restore-quarantine-{}.db",
                        crate::tasks::validation::id()
                    )),
                )?;
            }
            fs::rename(rescue, path)?;
        }
    }
    fs::remove_file(journal)?;
    if commit.exists() {
        fs::remove_file(commit)?;
    }
    Ok(())
}
fn remove_idle_sidecars(path: &Path) -> Result<(), ServiceError> {
    for suffix in ["-wal", "-shm"] {
        let p = PathBuf::from(format!("{}{suffix}", path.display()));
        if p.exists() {
            // A leftover WAL belongs to the quarantined candidate, never to the rollback
            // main file. Preserve it rather than mixing it into a different database.
            fs::rename(
                &p,
                p.with_file_name(format!(
                    ".restore-sidecar-{}{}",
                    crate::tasks::validation::id(),
                    suffix
                )),
            )?;
        }
    }
    Ok(())
}
fn close_database(db: Database) -> Result<(), (Database, ServiceError)> {
    let c = match db.connection.into_inner() {
        Ok(c) => c,
        Err(e) => e.into_inner(),
    };
    match c.close() {
        Ok(()) => Ok(()),
        Err((c, e)) => Err((
            Database {
                connection: Mutex::new(c),
            },
            e.into(),
        )),
    }
}
fn checkpoint(db: &Database) -> Result<(), ServiceError> {
    let c = db.connection.lock().map_err(|_| ServiceError::Lock)?;
    let busy: i64 = c.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get(0))?;
    if busy != 0 {
        return Err(ServiceError::Validation(
            "数据库仍被占用，恢复已取消，原数据保留。",
        ));
    }
    let mode: String = c.query_row("PRAGMA journal_mode=DELETE", [], |r| r.get(0))?;
    if mode != "delete" {
        return Err(ServiceError::Validation("无法关闭WAL，恢复已取消。"));
    }
    Ok(())
}
impl LocalService {
    pub(crate) fn restore_backup(&self, id: &str, confirmed: bool) -> Result<String, ServiceError> {
        if !confirmed {
            return Err(ServiceError::Validation(
                "恢复会替换当前全部数据，必须明确确认。",
            ));
        }
        let _gate = self.data_gate.lock().map_err(|_| ServiceError::Lock)?;
        let root = parent(&self.path)?;
        let old_journal = root.join("restore-journal.json");
        if old_journal.exists() {
            return Err(ServiceError::Validation(
                "上次恢复日志尚未清理，请重启应用后再恢复；当前数据保留。",
            ));
        }
        let dir = backup::directory(&self.path)?;
        let (selected, source) = backup::select(&dir, id)?;
        let schema = backup::validate(&backup::readonly(&source)?)?;
        if schema != selected.schema_version {
            return Err(ServiceError::Validation("备份元数据与数据库版本不一致。"));
        }
        if self.maintenance.swap(true, Ordering::SeqCst) {
            return Err(ServiceError::Validation("已有数据恢复操作。"));
        }
        struct Reset<'a>(&'a std::sync::atomic::AtomicBool);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::SeqCst);
            }
        }
        let _reset = Reset(&self.maintenance);
        let _exclusive = self.lifecycle.write().map_err(|_| ServiceError::Lock)?;
        if self.closed.load(Ordering::SeqCst) {
            return Err(ServiceError::Validation("服务已关闭。"));
        }
        let current = self.database()?;
        let safety = backup::create(&current, &self.path, &dir, "safety")?;
        let root = parent(&self.path)?;
        let restore_id = crate::tasks::validation::id();
        let staging = root.join(format!(".restore-staging-{restore_id}.db"));
        backup::online(&backup::readonly(&source)?, &staging)?;
        if backup::hash(&source)? != selected.sha256 {
            return Err(ServiceError::Validation(
                "备份在读取期间发生变化，恢复已取消，原数据保留。",
            ));
        }
        let staged = Database::open(&staging)?;
        let prepare = (|| {
            let old = current.connection.lock().map_err(|_| ServiceError::Lock)?;
            let (tr,sr,versions):(i64,i64,i64)=old.query_row("SELECT (SELECT revision FROM task_state),(SELECT revision FROM app_settings),MAX(COALESCE((SELECT MAX(version) FROM tasks),1),COALESCE((SELECT MAX(version) FROM categories),1))",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
            let data_version: i64 =
                old.query_row("SELECT version FROM data_settings WHERE id=1", [], |r| {
                    r.get(0)
                })?;
            let mut q=old.prepare("SELECT delivery_key,claimed_at FROM reminder_delivery_guard UNION SELECT json_array(COALESCE(s.series_id,'task:'||r.task_id),COALESCE(s.occurrence_date,''),r.slot,r.scheduled_at),r.claimed_at FROM reminders r LEFT JOIN task_schedule s ON s.task_id=r.task_id WHERE r.claimed_at IS NOT NULL")?;
            let guards = q
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            let mut c = staged.connection.lock().map_err(|_| ServiceError::Lock)?;
            let tx = c.transaction()?;
            tx.execute("UPDATE task_state SET revision=MAX(revision,?1)+1", [tr])?;
            tx.execute("UPDATE app_settings SET revision=MAX(revision,?1)+1", [sr])?;
            let base:i64=tx.query_row("SELECT MAX(?1,COALESCE((SELECT MAX(version) FROM tasks),1),COALESCE((SELECT MAX(version) FROM categories),1))+1",[versions],|r|r.get(0))?;
            tx.execute("UPDATE tasks SET version=?1", [base])?;
            tx.execute("UPDATE categories SET version=?1", [base])?;
            tx.execute(
                "UPDATE data_settings SET version=MAX(version,?1)+1",
                [data_version],
            )?;
            for (key, at) in guards {
                tx.execute("INSERT OR IGNORE INTO reminder_delivery_guard(delivery_key,claimed_at) VALUES(?1,?2)",params![key,at])?;
            }
            // Keep local backup inventory across restoration of an older database snapshot.
            for e in backup::list(&dir)? {
                tx.execute("INSERT OR REPLACE INTO backup_records(id,filename,kind,created_at,sha256,bytes,schema_version) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![e.id,e.filename,e.kind,e.created_at,e.sha256,i64::try_from(e.bytes).map_err(|_|ServiceError::Validation("备份文件过大。"))?,e.schema_version])?;
            }
            tx.commit()?;
            backup::validate(&c)?;
            Ok::<_, ServiceError>(())
        })();
        prepare?;
        staged.task_snapshot()?;
        staged.settings()?;
        staged.backup_settings()?;
        {
            let c = staged.connection.lock().map_err(|_| ServiceError::Lock)?;
            crate::schedule::validate_restored(&c)?;
        }
        checkpoint(&staged)?;
        close_database(staged).map_err(|(_, e)| e)?;
        checkpoint(&current)?;
        drop(current);
        let mut slot = self.database.lock().map_err(|_| ServiceError::Lock)?;
        let old = slot.take().ok_or(ServiceError::InvalidSettings)?;
        let old = match Arc::try_unwrap(old) {
            Ok(db) => db,
            Err(db) => {
                *slot = Some(db);
                return Err(ServiceError::Validation("数据库还有活动引用，恢复已取消。"));
            }
        };
        if let Err((db, e)) = close_database(old) {
            *slot = Some(Arc::new(db));
            return Err(e);
        }
        let rollback = root.join(format!(".restore-original-{restore_id}.db"));
        let j = Journal {
            id: restore_id.clone(),
            staging: staging
                .file_name()
                .ok_or(ServiceError::InvalidSettings)?
                .to_string_lossy()
                .into(),
            rollback: rollback
                .file_name()
                .ok_or(ServiceError::InvalidSettings)?
                .to_string_lossy()
                .into(),
            original_hash: backup::hash(&self.path)?,
            safety: safety.filename.clone(),
        };
        let journal = root.join("restore-journal.json");
        let replace = (|| {
            publish_record(
                &journal,
                &root.join(format!(".restore-journal-{restore_id}.partial")),
                &serde_json::to_vec(&j)?,
            )?;
            replace_closed(&self.path, &staging, &rollback)?;
            let loaded = Database::open(&self.path)?;
            let checked = (|| {
                loaded.task_snapshot()?;
                loaded.settings()?;
                loaded.backup_settings()?;
                Ok::<_, ServiceError>(())
            })();
            if let Err(e) = checked {
                let _ = checkpoint(&loaded);
                drop(loaded);
                return Err(e);
            }
            if let Err(e) = publish_record(
                &marker(root, &restore_id),
                &root.join(format!(".restore-commit-{restore_id}.partial")),
                b"committed",
            ) {
                let _ = checkpoint(&loaded);
                drop(loaded);
                return Err(e);
            }
            *slot = Some(Arc::new(loaded));
            self.epoch.fetch_add(1, Ordering::SeqCst);
            // If cleanup fails, the durable committed marker still prevents rollback.
            if fs::remove_file(&journal).is_ok() {
                let _ = fs::remove_file(marker(root, &restore_id));
            }
            Ok(())
        })();
        if let Err(error) = replace {
            // All new connections have dropped before moving any files back.
            if let Err(e) = recover_interrupted(&self.path) {
                log::error!("restore rollback needs startup recovery: {e}");
                return Err(ServiceError::Validation(
                    "恢复失败，回退文件和安全备份已保留；请重启应用重试恢复连接。",
                ));
            }
            *slot = Some(Arc::new(Database::open(&self.path)?));
            return Err(error);
        }
        Ok(safety.id)
    }
}
#[cfg(windows)]
fn replace_closed(original: &Path, new: &Path, rollback: &Path) -> Result<(), ServiceError> {
    use std::os::windows::ffi::OsStrExt;
    let wide = |p: &Path| {
        p.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<u16>>()
    };
    let a = wide(original);
    let b = wide(new);
    let c = wide(rollback);
    // All SQLite handles are closed. ReplaceFileW keeps an original-file rollback
    // on the same volume, with a durable journal covering partial Win32 failures.
    let ok = unsafe {
        windows_sys::Win32::Storage::FileSystem::ReplaceFileW(
            a.as_ptr(),
            b.as_ptr(),
            c.as_ptr(),
            0,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}
#[cfg(not(windows))]
fn replace_closed(original: &Path, new: &Path, rollback: &Path) -> Result<(), ServiceError> {
    fs::rename(original, rollback)?;
    fs::rename(new, original)?;
    Ok(())
}
