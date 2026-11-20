//! Serializable error contract returned by Tauri IPC commands.

use serde::Serialize;
use specta::Type;

/// Structured failure returned across the Tauri IPC boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    /// Stable error code identifying the failure class.
    pub code: &'static str,
    /// Human-readable message describing the failure.
    pub message: String,
}

/// Alias for Tauri IPC command results.
pub type Result<T> = std::result::Result<T, AppError>;

/// Constructs a structured [`AppError`] from a stable code and message.
pub fn error(code: &'static str, message: impl Into<String>) -> AppError {
    AppError {
        code,
        message: message.into(),
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        error("io", value.to_string())
    }
}
