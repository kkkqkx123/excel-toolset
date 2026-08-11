use axum::Json;
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::types::*;
use crate::http::response::ApiJson;

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
) -> ApiJson<WriteResult> {
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
        Ok(data) => ApiJson(ApiResponse::ok(Some(data))),
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}

pub async fn freeze_panes_clear(
    Json(req): Json<FreezePanesClearReq>,
) -> ApiJson<WriteResult> {
    let params = SecurityParams {
        dry_run: false,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::clear_freeze_panes(&req.path, &params, &req.sheet) {
        Ok(data) => ApiJson(ApiResponse::ok(Some(data))),
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}
