use axum::Json;
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct PivotTableCreateReq {
    pub path: String,
    pub config: PivotTableConfig,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn pivot_table_create(
    Json(req): Json<PivotTableCreateReq>,
) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::create_pivot_table(&req.path, &params, &req.config) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
