use axum::Json;
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct SlicerCreateReq {
    pub path: String,
    pub config: SlicerConfig,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn slicer_create(Json(req): Json<SlicerCreateReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::create_slicer(&req.path, &params, &req.config) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
