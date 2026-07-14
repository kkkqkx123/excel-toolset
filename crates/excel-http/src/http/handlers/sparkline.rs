use axum::Json;
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::features::sparkline;
use excel_core::types::*;
use excel_core::utils::cell_ref;

#[derive(Deserialize)]
pub struct SparklineAddReq {
    pub path: String,
    pub sheet: String,
    /// Source data range, e.g., "'Sheet1'!A1:E1"
    pub source_range: String,
    /// Target cell, e.g., "F1"
    pub target_cell: String,
    /// Sparkline type: line, column, winlose
    #[serde(default = "default_sparkline_type")]
    pub sparkline_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<u8>,
    #[serde(default)]
    pub dry_run: bool,
}

fn default_sparkline_type() -> String {
    "line".to_string()
}

#[derive(Deserialize)]
pub struct SparklineRemoveReq {
    pub path: String,
    pub sheet: String,
    pub target_cell: String,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn sparkline_add(Json(req): Json<SparklineAddReq>) -> Json<ApiResponse<WriteResult>> {
    let (target_row, target_col) = match cell_ref::parse_cell_ref(&req.target_cell) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::err(e)),
    };
    let st = sparkline::parse_sparkline_type(&req.sparkline_type);
    let config = SparklineConfig {
        sparkline_type: st,
        sheet: req.sheet,
        source_range: req.source_range,
        target_row,
        target_col,
        style: req.style,
    };
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::add_sparkline(&req.path, &params, &config) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn sparkline_remove(
    Json(req): Json<SparklineRemoveReq>,
) -> Json<ApiResponse<WriteResult>> {
    let (target_row, target_col) = match cell_ref::parse_cell_ref(&req.target_cell) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::err(e)),
    };
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::remove_sparkline(&req.path, &params, &req.sheet, target_row, target_col) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
