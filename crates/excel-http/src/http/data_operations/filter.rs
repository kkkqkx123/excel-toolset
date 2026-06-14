use axum::Json;
use serde::Deserialize;

use excel_core::operations;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct FilterReq {
    pub path: String,
    pub sheet: String,
    pub column: u16,
    pub operator: String,
    pub value: String,
}

#[derive(Deserialize)]
pub struct SortReq {
    pub path: String,
    pub sheet: String,
    pub column: u16,
    #[serde(default)]
    pub descending: bool,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct DedupReq {
    pub path: String,
    pub sheet: String,
    pub column: Option<u16>,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn data_filter(Json(req): Json<FilterReq>) -> Json<ApiResponse<Vec<Vec<CellData>>>> {
    let filter_op = match excel_core::utils::helpers::parse_filter_op(&req.operator) {
        Ok(op) => op,
        Err(e) => return Json(ApiResponse::err(e)),
    };
    let conditions = vec![FilterCondition {
        column: req.column,
        operator: filter_op,
        value: req.value,
    }];
    match operations::filter_rows(&req.path, &req.sheet, &conditions) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn data_sort(Json(req): Json<SortReq>) -> Json<ApiResponse<WriteResult>> {
    let sort_cols = vec![SortColumn {
        column: req.column,
        descending: req.descending,
    }];
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match operations::sort_sheet(&req.path, &params, &req.sheet, &sort_cols) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn data_dedup(Json(req): Json<DedupReq>) -> Json<ApiResponse<WriteResult>> {
    let cols = req.column.map(|c| vec![c]).unwrap_or_default();
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match operations::dedup_sheet(&req.path, &params, &req.sheet, &cols) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}