use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Account already exists: {0}")]
    AccountAlreadyExists(String),

    #[error("State file error: {0}")]
    StateFile(String),

    #[error("Process error: {0}")]
    Process(String),

    #[error("Path error: {0}")]
    Path(String),

    #[error("App not found: {0}")]
    AppNotFound(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("API error: {0}")]
    Api(String),
}

/// Serializable error for frontend consumption
#[derive(Serialize)]
pub struct SerializedError {
    pub message: String,
    pub kind: String,
}

impl From<AppError> for tauri::ipc::InvokeError {
    fn from(err: AppError) -> Self {
        let kind = match &err {
            AppError::Io(_) => "IO",
            AppError::Json(_) => "JSON",
            AppError::AccountNotFound(_) => "AccountNotFound",
            AppError::AccountAlreadyExists(_) => "AccountAlreadyExists",
            AppError::StateFile(_) => "StateFile",
            AppError::Process(_) => "Process",
            AppError::Path(_) => "Path",
            AppError::AppNotFound(_) => "AppNotFound",
            AppError::Session(_) => "Session",
            AppError::Api(_) => "Api",
        };
        let serialized = SerializedError {
            message: err.to_string(),
            kind: kind.to_string(),
        };
        tauri::ipc::InvokeError::from(serde_json::to_string(&serialized).unwrap_or_else(|_| {
            err.to_string()
        }))
    }
}

pub type AppResult<T> = Result<T, AppError>;
