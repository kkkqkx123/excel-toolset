use axum::Json;
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct FreezePanesSetReq {
    pub path: String,
    pub sheet: String,
    #[serde(default)]
    pub rows: u32,
    #[serde(default)]
    pub cols: u16,
}

#[derive(Deserialize)]
pub struct FreezePanesClearReq {
    pub path: String,
    pub sheet: String,
}

pub async fn freeze_panes_set(
    Json(req): Json<FreezePanesSetReq>,
) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: false,
        create_backup: true,
        file_path: req.path.clone(),
    };
    let config = FreezePanesConfig {
        sheet: req.sheet,
        rows: req.rows,
        cols: req.cols,
    };
    match excel_write::set_freeze_panes(&req.path, &params, &config) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn freeze_panes_clear(
    Json(req): Json<FreezePanesClearReq>,
) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: false,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::clear_freeze_panes(&req.path, &params, &req.sheet) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
