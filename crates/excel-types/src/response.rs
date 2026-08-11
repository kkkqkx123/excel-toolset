use serde::{Deserialize, Serialize};

use crate::diff::FileDiff;
use crate::error::AppError;
use crate::meta::BackupInfo;

/// Nested error detail, matching the documented envelope
/// `{ "success": false, "data": null, "error": { "code": ..., "message": ... } }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorDetail {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    /// Kept for backwards compatibility with older clients that read the flat
    /// `message` field. Omitted on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Always serialized (null on failure) so clients can rely on the key.
    pub data: Option<T>,
    /// Always serialized (null on success) so clients can rely on the key.
    pub error: Option<ApiErrorDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<FileDiff>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_info: Option<BackupInfo>,
    /// Transport hint only — never part of the JSON body. Lets the HTTP layer
    /// emit a truthful status code without re-deriving it from the error.
    #[serde(skip, default = "default_status")]
    pub status: u16,
}

fn default_status() -> u16 {
    200
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: Option<T>) -> Self {
        ApiResponse {
            success: true,
            message: None,
            data,
            error: None,
            file_hash: None,
            diff: None,
            backup_info: None,
            status: 200,
        }
    }

    pub fn err(e: AppError) -> Self {
        let code = e.error_code();
        let status = e.http_status();
        let message = e.to_string();
        ApiResponse {
            success: false,
            message: Some(message.clone()),
            data: None,
            error: Some(ApiErrorDetail {
                code: code.to_string(),
                message,
            }),
            file_hash: None,
            diff: None,
            backup_info: None,
            status,
        }
    }

    /// Machine-readable error code, or `None` when the response is a success.
    pub fn error_code(&self) -> Option<&str> {
        self.error.as_ref().map(|d| d.code.as_str())
    }
}

/// Maps a machine-readable error code to the HTTP status code the transport
/// layer should use. Kept here (rather than in `excel-http`) so CLI/MCP can
/// reuse the same classification when they need it.
///
/// Returns the numeric status so this crate stays free of an `axum`/`http`
/// dependency.
pub fn http_status_for_error_code(code: &str) -> u16 {
    match code {
        // Caller supplied something we cannot act on.
        "INVALID_INPUT"
        | "INVALID_ARGUMENT"
        | "INVALID_CELL_REF"
        | "INVALID_RANGE"
        | "INVALID_FILTER_OP"
        | "INVALID_CHART_TYPE"
        | "INVALID_TABLE_STYLE"
        | "INVALID_DATA_VALIDATION_TYPE"
        | "INVALID_PIVOT_AGGREGATION"
        | "DUCKDB_ERROR" => 400,
        // Target does not exist.
        "SHEET_NOT_FOUND" | "CELL_NOT_FOUND" => 404,
        // Target already exists / state conflict.
        "SHEET_ALREADY_EXISTS" => 409,
        // Understood but deliberately unavailable in this build.
        "FEATURE_NOT_ENABLED" | "VBA_NOT_SUPPORTED" => 501,
        // Everything else is a genuine server-side failure.
        _ => 500,
    }
}

impl AppError {
    /// Convenience wrapper around [`http_status_for_error_code`], with extra
    /// precision for `io::Error` (missing file → 404, denied → 403).
    pub fn http_status(&self) -> u16 {
        match self {
            AppError::Io(e) => io_status(e),
            // `open_workbook()` on a missing path surfaces as Calamine(Io(..)).
            AppError::Calamine(calamine::XlsxError::Io(e)) => io_status(e),
            // Malformed / non-xlsx payloads are the caller's fault, not ours.
            AppError::Calamine(_) => 400,
            _ => http_status_for_error_code(self.error_code()),
        }
    }
}

fn io_status(e: &std::io::Error) -> u16 {
    match e.kind() {
        std::io::ErrorKind::NotFound => 404,
        std::io::ErrorKind::PermissionDenied => 403,
        std::io::ErrorKind::AlreadyExists => 409,
        std::io::ErrorKind::InvalidInput | std::io::ErrorKind::InvalidData => 400,
        _ => 500,
    }
}
