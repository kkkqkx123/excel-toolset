use axum::Json;
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct AutoFilterSetReq {
    pub path: String,
    pub sheet: String,
    pub range: String,
}

#[derive(Deserialize)]
pub struct AutoFilterSheetReq {
    pub path: String,
    pub sheet: String,
}

pub async fn auto_filter_set(Json(req): Json<AutoFilterSetReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: false,
        create_backup: true,
        file_path: req.path.clone(),
    };
    let config = AutoFilterConfig {
        sheet: req.sheet,
        range: req.range,
    };
    match excel_write::set_auto_filter(&req.path, &params, &config) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn auto_filter_remove(
    Json(req): Json<AutoFilterSheetReq>,
) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: false,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::remove_auto_filter(&req.path, &params, &req.sheet) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn auto_filter_get(
    Json(req): Json<AutoFilterSheetReq>,
) -> Json<ApiResponse<AutoFilterInfo>> {
    match excel_write::get_auto_filter(&req.path, &req.sheet) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
