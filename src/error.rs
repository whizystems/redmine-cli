//! Unified error types with exit code mapping.

use std::process::ExitCode;
use thiserror::Error;

use crate::output::DryRunPreview;

/// Application-specific exit codes following the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppExitCode {
    /// Success
    Success = 0,
    /// Validation/argument errors
    Validation = 2,
    /// Authentication/configuration errors
    Auth = 3,
    /// Resource not found
    NotFound = 4,
    /// API/server/network errors
    ApiError = 5,
}

impl From<AppExitCode> for ExitCode {
    fn from(code: AppExitCode) -> Self {
        ExitCode::from(code as u8)
    }
}

/// Unified application error type.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        hint: Option<String>,
    },

    #[error("Configuration error: {message}")]
    Config {
        message: String,
        hint: Option<String>,
    },

    #[error("Authentication error: {message}")]
    Auth {
        message: String,
        hint: Option<String>,
    },

    #[error("Not found: {resource} #{id}")]
    NotFound {
        resource: String,
        id: String,
        hint: Option<String>,
    },

    #[error("API error: {message}")]
    Api {
        message: String,
        status: Option<u16>,
        hint: Option<String>,
    },

    #[error("Network error: {message}")]
    Network {
        message: String,
        hint: Option<String>,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    /// Not a failure: `--dry-run` built a write request and stopped before
    /// sending it. Carried on the error path so the command short-circuits;
    /// `main` prints it as a successful result and exits 0.
    #[error("Dry run: {} {} (no request sent)", .0.method, .0.path)]
    DryRun(Box<DryRunPreview>),
}

impl AppError {
    /// Get the exit code for this error.
    pub fn exit_code(&self) -> AppExitCode {
        match self {
            AppError::Validation { .. } => AppExitCode::Validation,
            AppError::Config { .. } => AppExitCode::Auth,
            AppError::Auth { .. } => AppExitCode::Auth,
            AppError::NotFound { .. } => AppExitCode::NotFound,
            AppError::Api { .. } => AppExitCode::ApiError,
            AppError::Network { .. } => AppExitCode::ApiError,
            AppError::Io(_) => AppExitCode::ApiError,
            AppError::Json(_) => AppExitCode::ApiError,
            AppError::Toml(_) => AppExitCode::Auth,
            AppError::DryRun(_) => AppExitCode::Success,
        }
    }

    /// Get the error code string for output.
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Validation { .. } => "VALIDATION_ERROR",
            AppError::Config { .. } => "CONFIG_ERROR",
            AppError::Auth { .. } => "AUTH_ERROR",
            AppError::NotFound { .. } => "NOT_FOUND",
            AppError::Api { .. } => "API_ERROR",
            AppError::Network { .. } => "NETWORK_ERROR",
            AppError::Io(_) => "IO_ERROR",
            AppError::Json(_) => "JSON_ERROR",
            AppError::Toml(_) => "CONFIG_ERROR",
            AppError::DryRun(_) => "DRY_RUN",
        }
    }

    /// Get the hint for this error, if any.
    pub fn hint(&self) -> Option<&str> {
        match self {
            AppError::Validation { hint, .. } => hint.as_deref(),
            AppError::Config { hint, .. } => hint.as_deref(),
            AppError::Auth { hint, .. } => hint.as_deref(),
            AppError::NotFound { hint, .. } => hint.as_deref(),
            AppError::Api { hint, .. } => hint.as_deref(),
            AppError::Network { hint, .. } => hint.as_deref(),
            _ => None,
        }
    }

    /// Create a dry-run outcome for a write request that was not sent.
    pub fn dry_run(method: &str, path: impl Into<String>, body: Option<serde_json::Value>) -> Self {
        AppError::DryRun(Box::new(DryRunPreview::new(method, path, body)))
    }

    /// Create a validation error.
    pub fn validation(message: impl Into<String>) -> Self {
        AppError::Validation {
            message: message.into(),
            hint: None,
        }
    }

    /// Create a validation error with hint.
    pub fn validation_with_hint(message: impl Into<String>, hint: impl Into<String>) -> Self {
        AppError::Validation {
            message: message.into(),
            hint: Some(hint.into()),
        }
    }

    /// Create a config error.
    pub fn config(message: impl Into<String>) -> Self {
        AppError::Config {
            message: message.into(),
            hint: None,
        }
    }

    /// Create a config error with hint.
    pub fn config_with_hint(message: impl Into<String>, hint: impl Into<String>) -> Self {
        AppError::Config {
            message: message.into(),
            hint: Some(hint.into()),
        }
    }

    /// Create an auth error.
    pub fn auth(message: impl Into<String>) -> Self {
        AppError::Auth {
            message: message.into(),
            hint: None,
        }
    }

    /// Create an auth error with hint.
    pub fn auth_with_hint(message: impl Into<String>, hint: impl Into<String>) -> Self {
        AppError::Auth {
            message: message.into(),
            hint: Some(hint.into()),
        }
    }

    /// Create a not found error.
    #[allow(dead_code)]
    pub fn not_found(resource: impl Into<String>, id: impl Into<String>) -> Self {
        AppError::NotFound {
            resource: resource.into(),
            id: id.into(),
            hint: None,
        }
    }

    /// Create a not found error with hint.
    pub fn not_found_with_hint(
        resource: impl Into<String>,
        id: impl Into<String>,
        hint: impl Into<String>,
    ) -> Self {
        AppError::NotFound {
            resource: resource.into(),
            id: id.into(),
            hint: Some(hint.into()),
        }
    }

    /// Create an API error.
    pub fn api(message: impl Into<String>, status: Option<u16>) -> Self {
        AppError::Api {
            message: message.into(),
            status,
            hint: None,
        }
    }

    /// Create an API error with hint.
    pub fn api_with_hint(
        message: impl Into<String>,
        status: Option<u16>,
        hint: impl Into<String>,
    ) -> Self {
        AppError::Api {
            message: message.into(),
            status,
            hint: Some(hint.into()),
        }
    }

    /// Create a network error.
    pub fn network(message: impl Into<String>) -> Self {
        AppError::Network {
            message: message.into(),
            hint: None,
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_code_validation() {
        let err = AppError::validation("test");
        assert_eq!(err.exit_code(), AppExitCode::Validation);
        assert_eq!(err.exit_code() as u8, 2);
    }

    #[test]
    fn test_exit_code_auth() {
        let err = AppError::auth("test");
        assert_eq!(err.exit_code(), AppExitCode::Auth);
        assert_eq!(err.exit_code() as u8, 3);
    }

    #[test]
    fn test_exit_code_config() {
        let err = AppError::config("test");
        assert_eq!(err.exit_code(), AppExitCode::Auth);
        assert_eq!(err.exit_code() as u8, 3);
    }

    #[test]
    fn test_exit_code_not_found() {
        let err = AppError::not_found("Issue", "123");
        assert_eq!(err.exit_code(), AppExitCode::NotFound);
        assert_eq!(err.exit_code() as u8, 4);
    }

    #[test]
    fn test_exit_code_api() {
        let err = AppError::api("test", Some(500));
        assert_eq!(err.exit_code(), AppExitCode::ApiError);
        assert_eq!(err.exit_code() as u8, 5);
    }

    #[test]
    fn test_exit_code_network() {
        let err = AppError::network("test");
        assert_eq!(err.exit_code(), AppExitCode::ApiError);
        assert_eq!(err.exit_code() as u8, 5);
    }

    #[test]
    fn test_error_code_strings() {
        assert_eq!(AppError::validation("test").code(), "VALIDATION_ERROR");
        assert_eq!(AppError::auth("test").code(), "AUTH_ERROR");
        assert_eq!(AppError::config("test").code(), "CONFIG_ERROR");
        assert_eq!(AppError::not_found("x", "1").code(), "NOT_FOUND");
        assert_eq!(AppError::api("test", None).code(), "API_ERROR");
        assert_eq!(AppError::network("test").code(), "NETWORK_ERROR");
    }

    #[test]
    fn test_hint() {
        let err = AppError::validation_with_hint("message", "hint text");
        assert_eq!(err.hint(), Some("hint text"));

        let err_no_hint = AppError::validation("message");
        assert_eq!(err_no_hint.hint(), None);
    }

    #[test]
    fn test_dry_run_is_success() {
        let err = AppError::dry_run("DELETE", "/relations/1.json", None);
        assert_eq!(err.exit_code(), AppExitCode::Success);
        assert_eq!(err.exit_code() as u8, 0);
        assert_eq!(err.code(), "DRY_RUN");
        assert_eq!(
            err.to_string(),
            "Dry run: DELETE /relations/1.json (no request sent)"
        );
    }

    #[test]
    fn test_error_display() {
        let err = AppError::not_found("Issue", "123");
        assert_eq!(err.to_string(), "Not found: Issue #123");
    }
}
