use axum::{Json, extract::Path};
use serde::Deserialize;

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

pub async fn file_info(Path(path): Path<String>) -> Json<ApiResponse<FileInfo>> {
    match excel_read::read_file_info(&path) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn file_create(Json(req): Json<CreateFileReq>) -> Json<ApiResponse<WriteResult>> {
    match excel_write::create_file(&req.path, &req.sheet) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn file_backup(Json(req): Json<BackupFileReq>) -> Json<ApiResponse<BackupInfo>> {
    let hash = match security::compute_file_hash(&req.path) {
        Ok(h) => h,
        Err(e) => return Json(ApiResponse::err(AppError::Io(e))),
    };
    match security::create_backup(&req.path, &hash) {
        Ok(backup) => {
            if let Some(ref out) = req.output {
                let _ = std::fs::copy(&backup.backup_path, out);
            }
            Json(ApiResponse::ok(Some(backup)))
        }
        Err(e) => Json(ApiResponse::err(AppError::Io(e))),
    }
}

#[derive(Deserialize)]
pub struct RollbackReq {
    pub path: String,
    pub backup_path: String,
}

pub async fn file_rollback(Json(req): Json<RollbackReq>) -> Json<ApiResponse<()>> {
    let backup_info = BackupInfo {
        backup_path: req.backup_path.clone(),
        timestamp: chrono::Utc::now(),
        operation: "manual".to_string(),
        file_hash: String::new(),
    };
    match security::rollback(&backup_info, &req.path) {
        Ok(()) => Json(ApiResponse::ok(None)),
        Err(e) => Json(ApiResponse::err(AppError::Io(e))),
    }
}