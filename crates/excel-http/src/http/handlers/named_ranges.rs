use axum::{Json, extract::Path};
use serde::Deserialize;

use excel_core::features::named_ranges;
use excel_core::types::*;

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
    Path(path): Path<String>,
) -> Json<ApiResponse<Vec<named_ranges::NamedRange>>> {
    match named_ranges::list_named_ranges(&path) {
        Ok(ranges) => Json(ApiResponse::ok(Some(ranges))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn get_named_range_value(
    Json(req): Json<GetNamedRangeValueReq>,
) -> Json<ApiResponse<Vec<Vec<CellData>>>> {
    match named_ranges::get_named_range_value(&req.path, &req.name) {
        Ok(Some(data)) => Json(ApiResponse::ok(Some(data))),
        Ok(None) => Json(ApiResponse::ok(None)),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn create_named_range(
    Json(req): Json<CreateNamedRangeReq>,
) -> Json<ApiResponse<WriteResult>> {
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
        Ok(result) => Json(ApiResponse::ok(Some(result))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn delete_named_range(
    Json(req): Json<DeleteNamedRangeReq>,
) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };

    match named_ranges::delete_named_range(&req.path, &req.name, &params) {
        Ok(result) => Json(ApiResponse::ok(Some(result))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}