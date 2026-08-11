use axum::Json;
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::types::*;
use crate::http::response::ApiJson;

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

pub async fn data_append_row(Json(req): Json<RowOpReq>) -> ApiJson<WriteResult> {
    let row: Vec<Vec<CellValue>> = vec![
        req.values
            .iter()
            .map(|v| excel_core::utils::helpers::parse_cell_value(v))
            .collect(),
    ];
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::append_rows(&req.path, &params, &req.sheet, &row) {
        Ok(data) => ApiJson(ApiResponse::ok(Some(data))),
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}

pub async fn data_insert_row(Json(req): Json<InsertRowReq>) -> ApiJson<WriteResult> {
    let row: Vec<Vec<CellValue>> = vec![
        req.values
            .iter()
            .map(|v| excel_core::utils::helpers::parse_cell_value(v))
            .collect(),
    ];
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::insert_rows(&req.path, &params, &req.sheet, req.row, &row) {
        Ok(data) => ApiJson(ApiResponse::ok(Some(data))),
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}

pub async fn data_delete_row(Json(req): Json<DeleteRowReq>) -> ApiJson<WriteResult> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::delete_rows(&req.path, &params, &req.sheet, req.row, req.row) {
        Ok(data) => ApiJson(ApiResponse::ok(Some(data))),
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}
