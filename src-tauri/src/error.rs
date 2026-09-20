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

    /// Post-localization shape for `NotFound`. Carries a locale-aware
    /// `message` produced by `services::user_messages::localize_not_found`
    /// at the command boundary, while preserving the original `resource`
    /// and `id` for structured logging.
    ///
    /// Never produced by services directly. The
    /// `From<AppError> for CommandError` conversion collapses this back
    /// into `CommandError::NotFound { message }` so the IPC wire shape
    /// is unchanged from the un-localized path.
    #[error("localized not_found: {resource} `{id}` ({message})")]
    LocalizedNotFound {
        resource: &'static str,
        id: String,
        message: String,
    },

    /// Post-localization shape for `DuplicateField`. Carries a
    /// locale-aware `message` while preserving `field` and `value` for
    /// the frontend structured-field checks (e.g. `kind === "duplicate_field"`
    /// and `detail.field === "barcode"`).
    ///
    /// The `#[error("…")]` text deliberately re-uses the canonical
    /// `uniqueness violation: …` prefix so the
    /// `uniqueness violation: barcode` defensive fallback regex in
    /// `src/lib/products.ts` continues to match if the helper chain is
    /// bypassed and the raw `AppError` reaches the UI via the
    /// `(String, CommandError)` tuple conversion.
    ///
    /// Never produced by services directly. The
    /// `From<AppError> for CommandError` conversion collapses this into
    /// `CommandError::DuplicateField { field, value, message }` so the
    /// IPC wire shape adds a localized `message` while preserving
    /// `field` and `value`.
    #[error("localized duplicate_field: {field} = `{value}` ({message})")]
    LocalizedDuplicateField {
        field: &'static str,
        value: String,
        message: String,
    },

    /// Post-localization shape for `InfrastructureError`. Carries a
    /// locale-aware `message` produced by
    /// `services::user_messages::localize_internal` so the UI receives
    /// the internal-error notice in the active locale while the original
    /// `InfrastructureError` detail stays in tracing/logs.
    ///
    /// Never produced by services directly. The
    /// `From<AppError> for CommandError` conversion collapses this into
    /// `CommandError::Internal { message }`.
    #[error("localized internal: {message}")]
    LocalizedInternal { message: String },
}

/// Infrastructure errors from database, I/O, or external systems.
#[derive(Debug, Error)]
pub enum InfrastructureError {
    #[error("database error")]
    Database(#[from] sqlx::Error),

    #[error("I/O error")]
    Io(#[from] std::io::Error),

    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    #[error("PDF error: {0}")]
    Pdf(#[from] printpdf::Error),

    #[error("serialization error")]
    Serialization(#[from] serde_json::Error),

    #[error("failed to resolve app data directory")]
    AppDataPath,

    #[error("backup validation failed: {0}")]
    BackupValidation(String),

    #[error("backup/restore I/O error: {0}")]
    BackupIo(String),
}

/// All internal errors bubble up through this enum.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("domain: {0}")]
    Domain(#[from] DomainError),

    #[error("infrastructure: {0}")]
    Infrastructure(#[from] InfrastructureError),
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Infrastructure(InfrastructureError::Database(e))
    }
}

/// User-safe error shape that is safe to return over the Tauri IPC boundary.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum CommandError {
    #[serde(rename = "not_found")]
    NotFound { message: String },

    /// Carries a structured `field` (frontend checks `field === "barcode"`)
    /// plus a localized `message` for the UI to render. The `message` is
    /// the same canonical English string
    /// (`"uniqueness violation: <field> = \`<value>\` already exists"`)
    /// unless `localize_duplicate_field` rewrote it at the command
    /// boundary; either way the `field` and `value` fields are preserved
    /// verbatim so structured dispatch keeps working.
    #[serde(rename = "duplicate_field")]
    DuplicateField {
        field: String,
        value: String,
        message: String,
    },

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
                    value: value.clone(),
                    message: format!("uniqueness violation: {field} = `{value}` already exists"),
                }
            }
            AppError::Domain(DomainError::LocalizedNotFound {
                resource,
                id,
                message,
            }) => CommandError::NotFound {
                // Preserve the original `<resource> `<id>` not found` shape
                // for legacy callers and the canonical English regex
                // checks (the resource label and id are surfaced verbatim,
                // so a future frontend i18n layer can re-parse them).
                message: if message.is_empty() {
                    format!("{resource} `{id}` not found")
                } else {
                    message
                },
            },
            AppError::Domain(DomainError::LocalizedDuplicateField {
                field,
                value,
                message,
            }) => CommandError::DuplicateField {
                field: field.to_string(),
                value,
                message,
            },
            AppError::Domain(DomainError::Validation { message }) => {
                CommandError::Validation { message }
            }
            AppError::Domain(DomainError::BusinessRule { message }) => {
                CommandError::BusinessRule { message }
            }
            AppError::Domain(DomainError::LocalizedInternal { message }) => {
                CommandError::Internal { message }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression guard: the `DomainError::DuplicateField` `Display` impl
    /// must continue to start with `uniqueness violation: ` so the
    /// frontend defensive regex `/^uniqueness violation: barcode/`
    /// (defined in `src/lib/products.ts`) keeps matching when the helper
    /// chain is bypassed and the raw `AppError` reaches the UI via the
    /// `(String, CommandError)` tuple conversion. A change to the
    /// `#[error("...")]` annotation would break that fallback path.
    #[test]
    fn duplicate_field_display_preserves_uniqueness_violation_prefix() {
        let err = DomainError::DuplicateField {
            field: "barcode",
            value: "7501234567890".to_string(),
        };
        let label = err.to_string();
        assert!(
            label.starts_with("uniqueness violation: barcode"),
            "DuplicateField Display must keep the `uniqueness violation: barcode` prefix for the frontend defensive regex; got `{label}`",
        );
    }

    /// Regression guard: the `DomainError::NotFound` `Display` impl must
    /// keep the `resource not found: <resource> \`<id>\`` shape so the
    /// `(String, CommandError)` tuple conversion preserves diagnostic
    /// detail in tracing/logs even after the new post-localization
    /// `LocalizedNotFound` variant was introduced.
    #[test]
    fn not_found_display_preserves_resource_id_shape() {
        let err = DomainError::NotFound {
            resource: "expiry_lot",
            id: "deadbeef".to_string(),
        };
        let label = err.to_string();
        assert_eq!(label, "resource not found: expiry_lot `deadbeef`");
    }

    /// Regression guard: `CommandError::DuplicateField` must serialize
    /// the new `message` field while preserving `field` and `value`. The
    /// frontend `isDuplicateFieldError` (in `src/lib/products.ts`)
    /// dispatches on `kind === "duplicate_field"` and
    /// `detail.field === "barcode"`; both must keep working after the
    /// wire change.
    #[test]
    fn command_error_duplicate_field_serializes_field_value_message() {
        let cmd = CommandError::DuplicateField {
            field: "barcode".to_string(),
            value: "7501234567890".to_string(),
            message: "uniqueness violation: barcode = `7501234567890` already exists".to_string(),
        };
        let json = serde_json::to_string(&cmd).expect("DuplicateField must serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(parsed["kind"], "duplicate_field");
        assert_eq!(parsed["detail"]["field"], "barcode");
        assert_eq!(parsed["detail"]["value"], "7501234567890");
        assert!(parsed["detail"]["message"]
            .as_str()
            .unwrap_or("")
            .starts_with("uniqueness violation: barcode"));
    }

    /// Regression guard: the canonical English path through
    /// `From<AppError> for CommandError>` (no localization helpers)
    /// must still produce a localized-shape `message` for
    /// `DuplicateField`. This keeps the frontend `humanizeError`
    /// (`src/lib/errors.ts`) non-empty for the un-helped path.
    #[test]
    fn canonical_duplicate_field_message_survives_un_helped_conversion() {
        let app_err = AppError::Domain(DomainError::DuplicateField {
            field: "sku",
            value: "WIDGET-1".to_string(),
        });
        let cmd: CommandError = app_err.into();
        match cmd {
            CommandError::DuplicateField { message, .. } => {
                assert!(message.contains("WIDGET-1"));
                assert!(message.starts_with("uniqueness violation: sku"));
            }
            other => panic!("expected CommandError::DuplicateField, got {other:?}"),
        }
    }
}
