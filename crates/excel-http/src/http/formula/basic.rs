use axum::Json;
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct FormulaSetReq {
    pub path: String,
    pub sheet: String,
    pub cell: String,
    pub formula: String,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct FormulaRefreshReq {
    pub path: String,
    pub sheet: String,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn formula_set(Json(req): Json<FormulaSetReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::set_formula(&req.path, &params, &req.sheet, &req.cell, &req.formula) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn formula_refresh(Json(req): Json<FormulaRefreshReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::refresh_formulas(&req.path, &params, &req.sheet) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}