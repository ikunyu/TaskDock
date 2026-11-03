use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("local data directory unavailable: {0}")]
    Io(#[from] std::io::Error),
    #[error("database operation failed: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("migration validation failed: {0}")]
    Migration(String),
    #[error("database lock poisoned")]
    Lock,
    #[error("invalid stored settings")]
    InvalidSettings,
    #[error("validation failed: {0}")]
    Validation(&'static str),
    #[error("entity not found")]
    NotFound,
    #[error("stale entity version")]
    Conflict,
    #[error("parent completion requires confirmation")]
    ConfirmIncomplete,
    #[error("category name already exists")]
    DuplicateCategory,
    #[error("audit serialization failed")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Serialize)]
pub struct AppError {
    pub code: &'static str,
    pub message: &'static str,
}

impl AppError {
    pub fn service(error: ServiceError) -> Self {
        match error {
            ServiceError::Validation(message) => Self {
                code: "VALIDATION",
                message,
            },
            ServiceError::NotFound => Self {
                code: "NOT_FOUND",
                message: "任务或分类已不存在，请刷新。",
            },
            ServiceError::Conflict => Self {
                code: "CONFLICT",
                message: "其他窗口已修改此内容。草稿已保留，请重新加载后再编辑。",
            },
            ServiceError::ConfirmIncomplete => Self {
                code: "CONFIRM_INCOMPLETE",
                message: "还有未完成的子任务，仍要完成父任务吗？子任务将保持原状态。",
            },
            ServiceError::DuplicateCategory => Self {
                code: "DUPLICATE_CATEGORY",
                message: "已存在同名分类。",
            },
            other => Self::internal(other),
        }
    }
    pub fn internal(error: impl std::fmt::Display) -> Self {
        log::error!("service failure: {error}");
        Self {
            code: "SERVICE_UNAVAILABLE",
            message: "本地服务暂不可用，请重试或查看应用日志。",
        }
    }

    pub fn forbidden() -> Self {
        Self {
            code: "FORBIDDEN",
            message: "当前窗口没有此操作权限。",
        }
    }
}
