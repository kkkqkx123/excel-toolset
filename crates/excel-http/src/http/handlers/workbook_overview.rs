use axum::Json;
use serde::Deserialize;

use crate::http::response::ApiJson;
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

pub async fn workbook_overview(Json(req): Json<OverviewReq>) -> ApiJson<serde_json::Value> {
    if req.blueprint {
        match workbook_overview::get_workbook_blueprint(&req.path) {
            Ok(bp) => match serde_json::to_value(bp) {
                Ok(v) => ApiJson(ApiResponse::ok(Some(v))),
                Err(e) => ApiJson(ApiResponse::err(AppError::Serialize(e.to_string()))),
            },
            Err(e) => ApiJson(ApiResponse::err(e)),
        }
    } else {
        match workbook_overview::get_workbook_overview(&req.path) {
            Ok(ov) => match serde_json::to_value(ov) {
                Ok(v) => ApiJson(ApiResponse::ok(Some(v))),
                Err(e) => ApiJson(ApiResponse::err(AppError::Serialize(e.to_string()))),
            },
            Err(e) => ApiJson(ApiResponse::err(e)),
        }
    }
}

pub async fn workbook_history(Json(req): Json<HistoryReq>) -> ApiJson<Vec<WorkbookHistoryEntry>> {
    match workbook_overview::list_workbook_history(&req.path) {
        Ok(h) => ApiJson(ApiResponse::ok(Some(h))),
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}

pub async fn sheet_overview(Json(req): Json<SheetOverviewReq>) -> ApiJson<serde_json::Value> {
    match workbook_overview::get_sheet_overview(&req.path, &req.sheet) {
        Ok(ov) => match serde_json::to_value(ov) {
            Ok(v) => ApiJson(ApiResponse::ok(Some(v))),
            Err(e) => ApiJson(ApiResponse::err(AppError::Serialize(e.to_string()))),
        },
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}
