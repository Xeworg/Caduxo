//! Shared error types for the Caduxo application.
//!
//! Errors are structured so that Tauri command handlers can convert them
//! into user-safe messages while preserving diagnostics in logs.

use serde::Serialize;
use thiserror::Error;

/// Domain-level errors that represent business-rule violations.
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("resource not found: {resource} `{id}`")]
    NotFound { resource: &'static str, id: String },

    #[error("uniqueness violation: {field} = `{value}` already exists")]
    DuplicateField { field: &'static str, value: String },

    #[error("validation error: {message}")]
    Validation { message: String },

    #[error("business rule: {message}")]
    BusinessRule { message: String },
}

/// Infrastructure errors from database, I/O, or external systems.
#[derive(Debug, Error)]
pub enum InfrastructureError {
    #[error("database error")]
    Database(#[from] sqlx::Error),

    #[error("I/O error")]
    Io(#[from] std::io::Error),

    #[error("serialization error")]
    Serialization(#[from] serde_json::Error),

    #[error("failed to resolve app data directory")]
    AppDataPath,
}

/// All internal errors bubble up through this enum.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("domain: {0}")]
    Domain(#[from] DomainError),

    #[error("infrastructure: {0}")]
    Infrastructure(#[from] InfrastructureError),
}

/// User-safe error shape that is safe to return over the Tauri IPC boundary.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum CommandError {
    #[serde(rename = "not_found")]
    NotFound { message: String },

    #[serde(rename = "duplicate_field")]
    DuplicateField { field: String, value: String },

    #[serde(rename = "validation")]
    Validation { message: String },

    #[serde(rename = "business_rule")]
    BusinessRule { message: String },

    #[serde(rename = "internal")]
    Internal { message: String },
}

impl From<AppError> for CommandError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::Domain(DomainError::NotFound { resource, id }) => CommandError::NotFound {
                message: format!("{resource} `{id}` not found"),
            },
            AppError::Domain(DomainError::DuplicateField { field, value }) => {
                CommandError::DuplicateField {
                    field: field.to_string(),
                    value,
                }
            }
            AppError::Domain(DomainError::Validation { message }) => {
                CommandError::Validation { message }
            }
            AppError::Domain(DomainError::BusinessRule { message }) => {
                CommandError::BusinessRule { message }
            }
            AppError::Infrastructure(_) => {
                // Logged at the call site; surface a generic message to the UI.
                CommandError::Internal {
                    message: "An internal error occurred. Please try again.".to_string(),
                }
            }
        }
    }
}

/// Converts an `AppError` into a `(String, CommandError)` tuple suitable for
/// `anyhow::Error` propagation through Tauri commands.
///
/// Usage in a Tauri command:
/// ```ignore
/// .map_err(|e: AppError| -> (String, CommandError) {
///     let cmd_err = e.clone().into();
///     let label = format!("{e}");
///     (label, cmd_err)
/// })
/// ```
impl From<AppError> for (String, CommandError) {
    fn from(err: AppError) -> Self {
        let label = err.to_string();
        let cmd_err: CommandError = err.into();
        (label, cmd_err)
    }
}
