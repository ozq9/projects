use serde::ser::{Serialize, SerializeStruct, Serializer};

use crate::ai::AiError;

/// Every command returns `AppResult<T>`. Errors reach the frontend as
/// `{ kind: "ai.auth", message: "..." }` so the UI can react to the kind.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("File error: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Ai(#[from] AiError),
    #[error("Credential Manager error: {0}")]
    Secrets(String),
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn kind(&self) -> String {
        match self {
            AppError::Db(_) => "db".into(),
            AppError::Io(_) => "io".into(),
            AppError::Ai(e) => format!("ai.{}", e.kind()),
            AppError::Secrets(_) => "secrets".into(),
            AppError::Invalid(_) => "invalid".into(),
            AppError::Internal(_) => "internal".into(),
        }
    }

    pub fn invalid(msg: impl Into<String>) -> Self {
        AppError::Invalid(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        AppError::Internal(msg.into())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("kind", &self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
