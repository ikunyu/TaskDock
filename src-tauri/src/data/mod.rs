pub(crate) mod backup;
mod commands;
mod export;
mod model;
mod query;
mod restore;
mod runtime;
pub use commands::*;
pub(crate) use restore::recover_interrupted;
pub use runtime::start;

use crate::{error::ServiceError, service::LocalService};
use model::*;
impl LocalService {
    pub fn data_status(&self) -> Result<DataStatus, ServiceError> {
        self.with_database(|db| {
            let settings = db.backup_settings()?;
            let backups = backup::list(&backup::directory(&self.path)?)?;
            let c = db.connection.lock().map_err(|_| ServiceError::Lock)?;
            let (last_success, last_attempt, last_error) = c.query_row(
                "SELECT last_success,last_attempt,last_error FROM data_settings WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            Ok(DataStatus {
                data_directory: self
                    .path
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .display()
                    .to_string(),
                settings,
                backups,
                last_success,
                last_attempt,
                last_error,
            })
        })
    }
    pub fn create_backup(&self, kind: &str) -> Result<BackupEntry, ServiceError> {
        let _gate = self.data_gate.lock().map_err(|_| ServiceError::Lock)?;
        self.with_database(|db| {
            let dir = backup::directory(&self.path)?;
            match backup::create(db, &self.path, &dir, kind) {
                Ok(e) => {
                    if kind == "auto" {
                        let keep = db.backup_settings()?.retention;
                        if let Err(error) = backup::prune(db, &dir, keep) {
                            log::warn!("backup published; retention cleanup failed: {error}");
                            db.backup_failure("备份已保存，但旧自动备份清理失败；文件已保留。")?;
                        }
                    }
                    Ok(e)
                }
                Err(error) => {
                    let _ = db.backup_failure(
                        "备份失败，请检查磁盘空间、目录权限或数据库占用；已有备份保留。",
                    );
                    Err(error)
                }
            }
        })
    }
}
