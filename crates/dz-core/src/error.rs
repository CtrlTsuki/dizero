//! Core error type. Every error carries a stable machine-readable `code`
//! so the UI can branch on it without parsing messages. Messages never
//! include credentials or secret material.

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Invalid(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("plugin error: {0}")]
    Plugin(String),
}

impl CoreError {
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::Db(_) => "db",
            CoreError::Json(_) => "json",
            CoreError::Io(_) => "io",
            CoreError::Invalid(_) => "invalid",
            CoreError::NotFound(_) => "not_found",
            CoreError::PermissionDenied(_) => "permission_denied",
            CoreError::Unsupported(_) => "unsupported",
            CoreError::Plugin(_) => "plugin",
        }
    }

    pub fn invalid(msg: impl Into<String>) -> Self {
        CoreError::Invalid(msg.into())
    }
}

pub type Result<T> = std::result::Result<T, CoreError>;

/// Serializable error shape returned across the IPC boundary.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct IpcError {
    pub code: String,
    pub message: String,
}

impl From<CoreError> for IpcError {
    fn from(e: CoreError) -> Self {
        IpcError { code: e.code().to_string(), message: e.to_string() }
    }
}
