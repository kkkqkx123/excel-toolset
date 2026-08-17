use axum::Json;
use serde::Deserialize;

use crate::http::response::ApiJson;
use excel_core::excel_read;
use excel_core::excel_write;
use excel_core::security;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct CreateFileReq {
    pub path: String,
    #[serde(default = "default_sheet")]
    pub sheet: String,
}

fn default_sheet() -> String {
    "Sheet1".into()
}

#[derive(Deserialize)]
pub struct BackupFileReq {
    pub path: String,
    pub output: Option<String>,
}

#[derive(Deserialize)]
pub struct FileInfoReq {
    pub path: String,
}

pub async fn file_info(Json(req): Json<FileInfoReq>) -> ApiJson<FileInfo> {
    match excel_read::read_file_info(&req.path) {
        Ok(data) => ApiJson(ApiResponse::ok(Some(data))),
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}

pub async fn file_create(Json(req): Json<CreateFileReq>) -> ApiJson<WriteResult> {
    match excel_write::create_file(&req.path, &req.sheet) {
        Ok(data) => ApiJson(ApiResponse::ok(Some(data))),
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}

pub async fn file_backup(Json(req): Json<BackupFileReq>) -> ApiJson<BackupInfo> {
    let hash = match security::compute_file_hash(&req.path) {
        Ok(h) => h,
        Err(e) => return ApiJson(ApiResponse::err(AppError::Io(e))),
    };
    match security::create_backup(&req.path, &hash) {
        Ok(backup) => {
            if let Some(ref out) = req.output {
                let _ = std::fs::copy(&backup.backup_path, out);
            }
            ApiJson(ApiResponse::ok(Some(backup)))
        }
        Err(e) => ApiJson(ApiResponse::err(AppError::Io(e))),
    }
}

#[derive(Deserialize)]
pub struct RollbackReq {
    pub path: String,
    pub backup_path: String,
}

pub async fn file_rollback(Json(req): Json<RollbackReq>) -> ApiJson<()> {
    let backup_info = BackupInfo {
        backup_path: req.backup_path.clone(),
        timestamp: chrono::Utc::now(),
        operation: "manual".to_string(),
        file_hash: String::new(),
    };
    match security::rollback(&backup_info, &req.path) {
        Ok(()) => ApiJson(ApiResponse::ok(None)),
        Err(e) => ApiJson(ApiResponse::err(AppError::Io(e))),
    }
}
