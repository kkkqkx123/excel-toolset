use axum::Json;
use serde::Deserialize;

use excel_core::excel_read;
use excel_core::excel_write;
use excel_core::types::*;
use excel_core::utils::helpers;

#[derive(Deserialize)]
pub struct CellReadReq {
    pub path: String,
    pub sheet: String,
    pub cell: String,
}

#[derive(Deserialize)]
pub struct CellWriteReq {
    pub path: String,
    pub sheet: String,
    pub cell: String,
    pub value: String,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn cell_read(Json(req): Json<CellReadReq>) -> Json<ApiResponse<CellData>> {
    let (row, col) = match excel_core::utils::cell_ref::parse_cell_ref(&req.cell) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::err(e)),
    };
    match excel_read::read_cell(&req.path, &req.sheet, row, col) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn cell_write(Json(req): Json<CellWriteReq>) -> Json<ApiResponse<WriteResult>> {
    let (row, col) = match excel_core::utils::cell_ref::parse_cell_ref(&req.cell) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::err(e)),
    };
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::write_cell(
        &req.path,
        &params,
        &req.sheet,
        row,
        col,
        &helpers::parse_cell_value(&req.value),
    ) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
