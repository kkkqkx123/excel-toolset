use axum::Json;
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct MergeReq {
    pub path: String,
    pub sheet: String,
    pub range: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn cell_merge(Json(req): Json<MergeReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::merge_cells(&req.path, &params, &req.sheet, &req.range, &req.value) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
