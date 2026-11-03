use crate::{
    db::{Database, SettingsSnapshot, Theme},
    error::ServiceError,
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, OnceLock, RwLock,
    },
};

/// The one process-wide service shared by all WebViews. Failed initialization is
/// retryable through bootstrap; it never replaces an unreadable database.
pub struct LocalService {
    pub(crate) path: PathBuf,
    pub(crate) closed: AtomicBool,
    pub(crate) lifecycle: RwLock<()>,
    pub(crate) database: Mutex<Option<Arc<Database>>>,
    pub(crate) maintenance: AtomicBool,
    pub(crate) epoch: AtomicU64,
    pub(crate) data_gate: Mutex<()>,
    legacy_directory: Option<PathBuf>,
    preparation: OnceLock<Result<(), &'static str>>,
}

impl LocalService {
    pub(crate) fn with_database<T>(
        &self,
        operation: impl FnOnce(&Database) -> Result<T, ServiceError>,
    ) -> Result<T, ServiceError> {
        if self.maintenance.load(Ordering::SeqCst) {
            return Err(ServiceError::Validation("数据恢复中，请稍后重试。"));
        }
        let _lifecycle = self.lifecycle.read().map_err(|_| ServiceError::Lock)?;
        if self.maintenance.load(Ordering::SeqCst) {
            return Err(ServiceError::Validation("数据恢复中，请稍后重试。"));
        }
        let database = self.database()?;
        operation(&database)
    }
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            closed: AtomicBool::new(false),
            lifecycle: RwLock::new(()),
            database: Mutex::new(None),
            maintenance: AtomicBool::new(false),
            epoch: AtomicU64::new(0),
            data_gate: Mutex::new(()),
            legacy_directory: None,
            preparation: OnceLock::new(),
        }
    }

    pub fn portable(path: PathBuf, legacy_directory: PathBuf) -> Self {
        let mut service = Self::new(path);
        service.legacy_directory = Some(legacy_directory);
        service
    }

    pub(crate) fn database(&self) -> Result<Arc<Database>, ServiceError> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(ServiceError::Validation("服务已关闭。"));
        }
        let mut guard = self.database.lock().map_err(|_| ServiceError::Lock)?;
        if let Some(database) = guard.as_ref() {
            return Ok(Arc::clone(database));
        }
        if let Some(legacy) = &self.legacy_directory {
            // A failed copy is not retried every two seconds by the scheduler.
            // Restart after resolving permission/space issues; old data remains.
            let prepared = self.preparation.get_or_init(|| {
                crate::storage::prepare(&self.path, legacy).map_err(|error| {
                    log::error!("portable data preparation: {error}");
                    match error {
                        ServiceError::Validation(message) => message,
                        _ => "无法准备软件目录下的data文件夹，请检查目录权限或磁盘空间后重启。原数据已保留，不会改回C盘。",
                    }
                })
            });
            if let Err(message) = prepared {
                return Err(ServiceError::Validation(message));
            }
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        crate::data::recover_interrupted(&self.path)?;
        let database = Arc::new(Database::open(&self.path)?);
        *guard = Some(Arc::clone(&database));
        log::info!(
            "local database initialized; schema {}",
            crate::db::SCHEMA_VERSION
        );
        Ok(database)
    }

    pub fn shutdown(&self) -> Result<(), ServiceError> {
        self.closed.store(true, Ordering::SeqCst);
        let _lifecycle = self.lifecycle.write().map_err(|_| ServiceError::Lock)?;
        let database = self.database.lock().map_err(|_| ServiceError::Lock)?.take();
        if let Some(db) = database {
            let c = db.connection.lock().map_err(|_| ServiceError::Lock)?;
            c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        }
        Ok(())
    }
    pub(crate) fn resume_after_update_error(&self) -> Result<(), ServiceError> {
        {
            let _lifecycle = self.lifecycle.write().map_err(|_| ServiceError::Lock)?;
            self.closed.store(false, Ordering::SeqCst);
        }
        self.settings().map(|_| ())
    }

    pub fn settings(&self) -> Result<SettingsSnapshot, ServiceError> {
        self.with_database(Database::settings)
    }

    pub fn set_theme(&self, theme: Theme) -> Result<SettingsSnapshot, ServiceError> {
        self.with_database(|db| db.set_theme(theme))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_database_is_preserved_on_repeated_bootstrap() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("existing.sqlite3");
        let original = b"not a valid sqlite database - do not replace me";
        std::fs::write(&path, original).unwrap();
        let service = LocalService::new(path.clone());
        assert!(service.settings().is_err());
        assert!(service.settings().is_err());
        assert_eq!(std::fs::read(path).unwrap(), original);
    }

    #[test]
    fn initialization_can_retry_after_directory_failure() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("data");
        std::fs::write(&parent, "test-only directory blocker").unwrap();
        let service = LocalService::new(parent.join("test.sqlite3"));
        assert!(service.settings().is_err());
        std::fs::remove_file(&parent).unwrap();
        assert_eq!(service.settings().unwrap().theme, Theme::System);
    }
    #[test]
    fn shutdown_waits_for_active_request_before_checkpoint_and_rejects_new_requests() {
        let dir = tempfile::tempdir().unwrap();
        let service = Arc::new(LocalService::new(dir.path().join("drain.sqlite3")));
        service.settings().unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (finish_tx, finish_rx) = std::sync::mpsc::channel();
        let writer_service = service.clone();
        let writer = std::thread::spawn(move || {
            writer_service.with_database(|db| {
                started_tx.send(()).unwrap();
                finish_rx.recv().unwrap();
                db.create_task(crate::tasks::TaskDraft {
                    title: "退出前完成保存".into(),
                    ..Default::default()
                })
            })
        });
        started_rx.recv().unwrap();
        let shutdown_service = service.clone();
        let shutdown = std::thread::spawn(move || shutdown_service.shutdown());
        while !service.closed.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
        finish_tx.send(()).unwrap();
        writer.join().unwrap().unwrap();
        shutdown.join().unwrap().unwrap();
        assert!(service.settings().is_err());
        let reopened = Database::open(&dir.path().join("drain.sqlite3")).unwrap();
        assert_eq!(
            reopened.task_snapshot().unwrap().tasks[0].title,
            "退出前完成保存"
        );
    }
}
