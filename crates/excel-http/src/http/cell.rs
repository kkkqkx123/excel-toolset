use axum::{Json, extract::Path};
use serde::Deserialize;

use excel_core::excel_read;
use excel_core::excel_write;
use excel_core::helpers;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct CellWriteReq {
    pub path: String,
    pub sheet: String,
    pub cell: String,
    pub value: String,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn cell_read(
    Path((path, sheet, cell)): Path<(String, String, String)>,
) -> Json<ApiResponse<CellData>> {
    let (row, col) = match excel_core::cell_ref::parse_cell_ref(&cell) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::err(e)),
    };
    match excel_read::read_cell(&path, &sheet, row, col) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn cell_write(Json(req): Json<CellWriteReq>) -> Json<ApiResponse<WriteResult>> {
    let (row, col) = match excel_core::cell_ref::parse_cell_ref(&req.cell) {
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
