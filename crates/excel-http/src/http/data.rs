use axum::Json;
use serde::Deserialize;

use excel_core::api;
use excel_core::excel_data;
use excel_core::helpers;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct RowOpReq {
    pub path: String,
    pub sheet: String,
    pub values: Vec<String>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct InsertRowReq {
    pub path: String,
    pub sheet: String,
    pub row: u32,
    pub values: Vec<String>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct DeleteRowReq {
    pub path: String,
    pub sheet: String,
    pub row: u32,
    #[serde(default)]
    pub dry_run: bool,
}

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

#[derive(Deserialize)]
pub struct SqlReq {
    pub path: String,
    pub sheet: String,
    pub query: String,
}

pub async fn data_append_row(Json(req): Json<RowOpReq>) -> Json<ApiResponse<WriteResult>> {
    let row: Vec<Vec<CellValue>> = vec![
        req.values
            .iter()
            .map(|v| helpers::parse_cell_value(v))
            .collect(),
    ];
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_data::append_rows(&req.path, &params, &req.sheet, &row) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn data_insert_row(Json(req): Json<InsertRowReq>) -> Json<ApiResponse<WriteResult>> {
    let row: Vec<Vec<CellValue>> = vec![
        req.values
            .iter()
            .map(|v| helpers::parse_cell_value(v))
            .collect(),
    ];
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_data::insert_rows(&req.path, &params, &req.sheet, req.row, &row) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn data_delete_row(Json(req): Json<DeleteRowReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_data::delete_rows(&req.path, &params, &req.sheet, req.row, req.row) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn data_filter(Json(req): Json<FilterReq>) -> Json<ApiResponse<Vec<Vec<CellData>>>> {
    let filter_op = match helpers::parse_filter_op(&req.operator) {
        Ok(op) => op,
        Err(e) => return Json(ApiResponse::err(e)),
    };
    let conditions = vec![FilterCondition {
        column: req.column,
        operator: filter_op,
        value: req.value,
    }];
    match api::filter_rows(&req.path, &req.sheet, &conditions) {
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
    match api::sort_sheet(&req.path, &params, &req.sheet, &sort_cols) {
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
    match api::dedup_sheet(&req.path, &params, &req.sheet, &cols) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn data_sql(Json(req): Json<SqlReq>) -> Json<ApiResponse<Vec<Vec<CellData>>>> {
    match api::sql_query(&req.path, &req.sheet, &req.query) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
