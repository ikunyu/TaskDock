use super::model::*;
use crate::{
    db::{Database, SCHEMA_VERSION},
    error::ServiceError,
    tasks::validation,
};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{
    backup::{Backup, StepResult},
    params, Connection, OpenFlags,
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration as Wait,
};

pub(crate) fn hash(path: &Path) -> Result<String, ServiceError> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
pub(crate) fn validate(c: &Connection) -> Result<u32, ServiceError> {
    c.pragma_update(None, "trusted_schema", "OFF")?;
    let version = crate::db::validate_backup_ledger(c)?;
    let mut q = c.prepare("PRAGMA integrity_check")?;
    let results = q
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    if results != ["ok"] || c.prepare("PRAGMA foreign_key_check")?.exists([])? {
        return Err(ServiceError::Validation(
            "备份完整性或外键检查失败，拒绝恢复。",
        ));
    }
    Ok(version)
}
pub(crate) fn readonly(path: &Path) -> Result<Connection, ServiceError> {
    let c = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    c.busy_timeout(Wait::from_secs(5))?;
    c.pragma_update(None, "query_only", "ON")?;
    Ok(c)
}
pub(crate) fn online(source: &Connection, path: &Path) -> Result<(), ServiceError> {
    let f = OpenOptions::new().write(true).create_new(true).open(path)?;
    drop(f);
    let result = (|| {
        let mut target = Connection::open(path)?;
        target.busy_timeout(Wait::from_secs(5))?;
        {
            let backup = Backup::new(source, &mut target)?;
            let started = std::time::Instant::now();
            loop {
                if started.elapsed() > Wait::from_secs(120) {
                    return Err(ServiceError::Validation(
                        "备份超时，原数据库和已有备份均保留。",
                    ));
                }
                match backup.step(128)? {
                    StepResult::Done => break,
                    StepResult::More => std::thread::yield_now(),
                    _ => std::thread::sleep(Wait::from_millis(10)),
                }
            }
        }
        target.pragma_update(None, "journal_mode", "DELETE")?;
        validate(&target)?;
        target.close().map_err(|(_, e)| ServiceError::from(e))?;
        // FlushFileBuffers on Windows requires a writable handle.
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?
            .sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(path);
    }
    result
}
pub(crate) fn directory(db: &Path) -> Result<PathBuf, ServiceError> {
    Ok(db
        .parent()
        .ok_or(ServiceError::Validation("数据库目录无效。"))?
        .join("backups"))
}
pub(crate) fn list(dir: &Path) -> Result<Vec<BackupEntry>, ServiceError> {
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut entries = vec![];
    for e in fs::read_dir(dir)? {
        let p = e?.path();
        if p.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if fs::symlink_metadata(&p)?.file_type().is_symlink() {
            continue;
        }
        let Ok(file) = File::open(&p) else {
            continue;
        };
        let Ok(entry) = serde_json::from_reader::<_, BackupEntry>(file.take(64 * 1024)) else {
            continue;
        };
        if valid_name(&entry)
            && p.file_name().and_then(|s| s.to_str()) == Some(&format!("{}.json", entry.filename))
            && dir.join(&entry.filename).is_file()
        {
            entries.push(entry);
        }
    }
    entries.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then(b.filename.cmp(&a.filename))
    });
    Ok(entries)
}
fn valid_name(e: &BackupEntry) -> bool {
    e.filename.starts_with("taskdock-backup-")
        && e.filename.ends_with(".db")
        && e.filename
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
        && uuid::Uuid::parse_str(&e.id).is_ok()
        && ["auto", "manual", "safety"].contains(&e.kind.as_str())
        && e.sha256.len() == 64
        && e.sha256.bytes().all(|b| b.is_ascii_hexdigit())
}
pub(crate) fn select(dir: &Path, id: &str) -> Result<(BackupEntry, PathBuf), ServiceError> {
    validation::validate_id(id)?;
    let entry = list(dir)?
        .into_iter()
        .find(|e| e.id == id)
        .ok_or(ServiceError::NotFound)?;
    let path = dir.join(&entry.filename);
    if fs::symlink_metadata(&path)?.file_type().is_symlink()
        || path.canonicalize()?.parent() != Some(dir.canonicalize()?.as_path())
    {
        return Err(ServiceError::Validation("备份路径无效。"));
    }
    if fs::metadata(&path)?.len() != entry.bytes || hash(&path)? != entry.sha256 {
        return Err(ServiceError::Validation("备份校验和不匹配，文件可能损坏。"));
    }
    Ok((entry, path))
}
pub(crate) fn create(
    db: &Database,
    live_path: &Path,
    dir: &Path,
    kind: &str,
) -> Result<BackupEntry, ServiceError> {
    fs::create_dir_all(dir)?;
    let id = validation::id();
    let created = validation::now();
    let filename = format!(
        "taskdock-backup-{}-{}.db",
        Utc::now().format("%Y%m%d-%H%M%S"),
        id
    );
    let temp = dir.join(format!(".{id}.partial"));
    let published = dir.join(&filename);
    {
        // Scoped read handle to the SAME database, owned by LocalService's
        // lifecycle lease. Incremental backup does not monopolize task writes.
        let source = readonly(live_path)?;
        online(&source, &temp)?;
    }
    let result = (|| {
        let entry = BackupEntry {
            id,
            filename,
            kind: kind.into(),
            created_at: created,
            sha256: hash(&temp)?,
            bytes: fs::metadata(&temp)?.len(),
            schema_version: SCHEMA_VERSION,
        };
        // Unique names prevent overwrite; publication occurs on the same volume.
        fs::rename(&temp, &published)?;
        let manifest = dir.join(format!("{}.json", entry.filename));
        let manifest_temp = dir.join(format!(".{}.manifest", entry.id));
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&manifest_temp)?;
        f.write_all(&serde_json::to_vec_pretty(&entry)?)?;
        f.sync_all()?;
        drop(f);
        fs::rename(manifest_temp, manifest)?;
        let mut c = db.connection.lock().map_err(|_| ServiceError::Lock)?;
        let tx = c.transaction()?;
        tx.execute("INSERT INTO backup_records(id,filename,kind,created_at,sha256,bytes,schema_version) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![entry.id,entry.filename,entry.kind,entry.created_at,entry.sha256,i64::try_from(entry.bytes).map_err(|_|ServiceError::Validation("备份文件过大。"))?,entry.schema_version])?;
        tx.execute(
            "UPDATE data_settings SET last_success=?1,last_attempt=?1,last_error=NULL WHERE id=1",
            [&entry.created_at],
        )?;
        tx.commit()?;
        Ok(entry)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
pub(crate) fn prune(db: &Database, dir: &Path, keep: i64) -> Result<(), ServiceError> {
    let entries = list(dir)?
        .into_iter()
        .filter(|e| e.kind == "auto")
        .collect::<Vec<_>>();
    // Before deleting any old auto backup, validate the newest completed one.
    let Some(newest) = entries.first() else {
        return Ok(());
    };
    let (_, path) = select(dir, &newest.id)?;
    validate(&readonly(&path)?)?;
    for entry in entries.iter().skip(keep.max(1) as usize) {
        let (_, path) = select(dir, &entry.id)?;
        fs::remove_file(path)?;
        fs::remove_file(dir.join(format!("{}.json", entry.filename)))?;
        db.connection
            .lock()
            .map_err(|_| ServiceError::Lock)?
            .execute("DELETE FROM backup_records WHERE id=?1", [&entry.id])?;
    }
    Ok(())
}
impl Database {
    pub fn backup_settings(&self) -> Result<BackupSettings, ServiceError> {
        Ok(self
            .connection
            .lock()
            .map_err(|_| ServiceError::Lock)?
            .query_row(
                "SELECT auto_backup,interval_hours,retention,version FROM data_settings WHERE id=1",
                [],
                |r| {
                    Ok(BackupSettings {
                        auto_backup: r.get(0)?,
                        interval_hours: r.get(1)?,
                        retention: r.get(2)?,
                        version: r.get(3)?,
                    })
                },
            )?)
    }
    pub fn save_backup_settings(&self, s: BackupSettings) -> Result<(), ServiceError> {
        if !(1..=8760).contains(&s.interval_hours) || !(1..=365).contains(&s.retention) {
            return Err(ServiceError::Validation(
                "备份周期为1–8760小时，保留数量为1–365份。",
            ));
        }
        let mut c = self.connection.lock().map_err(|_| ServiceError::Lock)?;
        let tx = c.transaction()?;
        if tx.execute("UPDATE data_settings SET auto_backup=?1,interval_hours=?2,retention=?3,version=version+1 WHERE id=1 AND version=?4",params![s.auto_backup,s.interval_hours,s.retention,s.version])?!=1{return Err(ServiceError::Conflict);}
        tx.execute("UPDATE app_settings SET revision=revision+1,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=1", [])?;
        tx.commit()?;
        Ok(())
    }
    pub fn backup_due(&self) -> Result<bool, ServiceError> {
        let s = self.backup_settings()?;
        if !s.auto_backup {
            return Ok(false);
        }
        let c = self.connection.lock().map_err(|_| ServiceError::Lock)?;
        let (last, attempt): (Option<String>, Option<String>) = c.query_row(
            "SELECT last_success,last_attempt FROM data_settings WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let now = Utc::now();
        let parse = |s: Option<String>| {
            s.and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc))
        };
        Ok(
            parse(last).is_none_or(|d| now - d >= Duration::hours(s.interval_hours))
                && parse(attempt).is_none_or(|d| now - d >= Duration::minutes(15)),
        )
    }
    pub fn backup_failure(&self, error: &str) -> Result<(), ServiceError> {
        self.connection
            .lock()
            .map_err(|_| ServiceError::Lock)?
            .execute(
                "UPDATE data_settings SET last_attempt=?1,last_error=?2 WHERE id=1",
                params![validation::now(), error],
            )?;
        Ok(())
    }
}
