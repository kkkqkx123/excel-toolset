use axum::Json;
use serde::Deserialize;

use excel_core::features::workbook_overview;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct OverviewReq {
    pub path: String,
    #[serde(default)]
    pub blueprint: bool,
}

#[derive(Deserialize)]
pub struct SheetOverviewReq {
    pub path: String,
    pub sheet: String,
}

#[derive(Deserialize)]
pub struct HistoryReq {
    pub path: String,
}

pub async fn workbook_overview(
    Json(req): Json<OverviewReq>,
) -> Json<ApiResponse<serde_json::Value>> {
    if req.blueprint {
        match workbook_overview::get_workbook_blueprint(&req.path) {
            Ok(bp) => match serde_json::to_value(bp) {
                Ok(v) => Json(ApiResponse::ok(Some(v))),
                Err(e) => Json(ApiResponse::err(AppError::Serialize(e.to_string()))),
            },
            Err(e) => Json(ApiResponse::err(e)),
        }
    } else {
        match workbook_overview::get_workbook_overview(&req.path) {
            Ok(ov) => match serde_json::to_value(ov) {
                Ok(v) => Json(ApiResponse::ok(Some(v))),
                Err(e) => Json(ApiResponse::err(AppError::Serialize(e.to_string()))),
            },
            Err(e) => Json(ApiResponse::err(e)),
        }
    }
}

pub async fn workbook_history(
    Json(req): Json<HistoryReq>,
) -> Json<ApiResponse<Vec<WorkbookHistoryEntry>>> {
    match workbook_overview::list_workbook_history(&req.path) {
        Ok(h) => Json(ApiResponse::ok(Some(h))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn sheet_overview(
    Json(req): Json<SheetOverviewReq>,
) -> Json<ApiResponse<serde_json::Value>> {
    match workbook_overview::get_sheet_overview(&req.path, &req.sheet) {
        Ok(ov) => match serde_json::to_value(ov) {
            Ok(v) => Json(ApiResponse::ok(Some(v))),
            Err(e) => Json(ApiResponse::err(AppError::Serialize(e.to_string()))),
        },
        Err(e) => Json(ApiResponse::err(e)),
    }
}
