use axum::Json;
use serde::Deserialize;

use crate::http::response::ApiJson;
use excel_core::features::named_ranges;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct ListNamedRangesReq {
    pub path: String,
}

#[derive(Deserialize)]
pub struct GetNamedRangeValueReq {
    pub path: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct CreateNamedRangeReq {
    pub path: String,
    pub name: String,
    pub range: String,
    pub sheet: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct DeleteNamedRangeReq {
    pub path: String,
    pub name: String,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn list_named_ranges(
    Json(req): Json<ListNamedRangesReq>,
) -> ApiJson<Vec<named_ranges::NamedRange>> {
    match named_ranges::list_named_ranges(&req.path) {
        Ok(ranges) => ApiJson(ApiResponse::ok(Some(ranges))),
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}

pub async fn get_named_range_value(
    Json(req): Json<GetNamedRangeValueReq>,
) -> ApiJson<Vec<Vec<CellData>>> {
    match named_ranges::get_named_range_value(&req.path, &req.name) {
        Ok(Some(data)) => ApiJson(ApiResponse::ok(Some(data))),
        Ok(None) => ApiJson(ApiResponse::ok(None)),
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}

pub async fn create_named_range(Json(req): Json<CreateNamedRangeReq>) -> ApiJson<WriteResult> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };

    match named_ranges::create_named_range(
        &req.path,
        &req.name,
        &req.range,
        req.sheet.as_deref(),
        &params,
    ) {
        Ok(result) => ApiJson(ApiResponse::ok(Some(result))),
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}

pub async fn delete_named_range(Json(req): Json<DeleteNamedRangeReq>) -> ApiJson<WriteResult> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };

    match named_ranges::delete_named_range(&req.path, &req.name, &params) {
        Ok(result) => ApiJson(ApiResponse::ok(Some(result))),
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}
