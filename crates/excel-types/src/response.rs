use serde::{Deserialize, Serialize};

use crate::diff::FileDiff;
use crate::error::AppError;
use crate::meta::BackupInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<FileDiff>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_info: Option<BackupInfo>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: Option<T>) -> Self {
        ApiResponse {
            success: true,
            message: String::new(),
            error_code: None,
            file_hash: None,
            data,
            diff: None,
            backup_info: None,
        }
    }

    pub fn err(e: AppError) -> Self {
        let code = e.error_code();
        ApiResponse {
            success: false,
            message: e.to_string(),
            error_code: Some(code),
            file_hash: None,
            data: None,
            diff: None,
            backup_info: None,
        }
    }
}