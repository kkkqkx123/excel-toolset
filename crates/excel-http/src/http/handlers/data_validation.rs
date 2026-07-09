use axum::Json;
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct DataValidationAddReq {
    pub path: String,
    pub sheet: String,
    pub config: DataValidationConfig,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct DataValidationRemoveReq {
    pub path: String,
    pub sheet: String,
    pub range: String,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn data_validation_add(
    Json(req): Json<DataValidationAddReq>,
) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::add_data_validation(&req.path, &params, &req.sheet, &req.config) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn data_validation_remove(
    Json(req): Json<DataValidationRemoveReq>,
) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::remove_data_validation(&req.path, &params, &req.sheet, &req.range) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
