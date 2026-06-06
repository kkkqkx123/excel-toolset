use axum::Json;
use serde::Deserialize;

use excel_core::features::vba_util;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct VbaExportReq {
    pub path: String,
    pub output: String,
}

#[derive(Deserialize)]
pub struct VbaImportReq {
    pub path: String,
    pub vba_file: String,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn vba_export(Json(req): Json<VbaExportReq>) -> Json<ApiResponse<String>> {
    match vba_util::export_vba(&req.path) {
        Ok(data) => {
            if let Err(e) = std::fs::write(&req.output, &data) {
                return Json(ApiResponse::err(AppError::Io(e)));
            }
            Json(ApiResponse::ok(Some(format!("Exported to {}", req.output))))
        }
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn vba_import(Json(req): Json<VbaImportReq>) -> Json<ApiResponse<WriteResult>> {
    let data = match std::fs::read(&req.vba_file) {
        Ok(d) => d,
        Err(e) => return Json(ApiResponse::err(AppError::Io(e))),
    };
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match vba_util::import_vba(&req.path, &params, &data) {
        Ok(result) => Json(ApiResponse::ok(Some(result))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}