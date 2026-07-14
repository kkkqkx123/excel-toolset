use axum::Json;
use serde::Deserialize;

use excel_core::excel_read;
use excel_core::excel_write;
use excel_core::types::*;
use excel_core::utils::helpers;

#[derive(Deserialize)]
pub struct RangeReadReq {
    pub path: String,
    pub sheet: String,
    pub range: String,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default)]
    pub truncate: Option<usize>,
}

fn default_mode() -> String {
    "detailed".into()
}

#[derive(Deserialize)]
pub struct RangeWriteReq {
    pub path: String,
    pub sheet: String,
    pub range: String,
    pub data: Vec<Vec<serde_json::Value>>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct RangeClearReq {
    pub path: String,
    pub sheet: String,
    pub range: String,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct RangeWriteCsvReq {
    pub path: String,
    pub sheet: String,
    pub range: String,
    pub csv_path: String,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn range_read(Json(req): Json<RangeReadReq>) -> Json<ApiResponse<ReadRangeResult>> {
    let mode = match req.mode.as_str() {
        "compact" => OutputMode::Compact,
        "csv" => OutputMode::Csv,
        _ => OutputMode::Detailed,
    };
    let options = ReadRangeOptions {
        mode,
        truncate: req.truncate,
        include_context: Some(false),
        context_size: Some(3),
    };
    match excel_read::read_range_with_options(&req.path, &req.sheet, &req.range, &options) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn range_write(Json(req): Json<RangeWriteReq>) -> Json<ApiResponse<WriteResult>> {
    let values: Vec<Vec<CellValue>> = req
        .data
        .iter()
        .map(|row: &Vec<serde_json::Value>| {
            row.iter()
                .map(|v: &serde_json::Value| helpers::json_val_to_cell_value(v))
                .collect()
        })
        .collect();
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::write_range(&req.path, &params, &req.sheet, &req.range, &values) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn range_clear(Json(req): Json<RangeClearReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::clear_range(&req.path, &params, &req.sheet, &req.range) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn range_write_from_csv(
    Json(req): Json<RangeWriteCsvReq>,
) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::write_range_from_csv(
        &req.path,
        &params,
        &req.sheet,
        &req.range,
        &req.csv_path,
    ) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
