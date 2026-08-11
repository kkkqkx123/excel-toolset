use axum::Json;
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::types::*;
use crate::http::response::ApiJson;

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
) -> ApiJson<WriteResult> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::add_data_validation(&req.path, &params, &req.sheet, &req.config) {
        Ok(data) => ApiJson(ApiResponse::ok(Some(data))),
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}

pub async fn data_validation_remove(
    Json(req): Json<DataValidationRemoveReq>,
) -> ApiJson<WriteResult> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::remove_data_validation(&req.path, &params, &req.sheet, &req.range) {
        Ok(data) => ApiJson(ApiResponse::ok(Some(data))),
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}
