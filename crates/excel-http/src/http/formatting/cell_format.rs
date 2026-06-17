use axum::Json;
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct FormatSetReq {
    pub path: String,
    pub sheet: String,
    pub range: String,
    pub style: Style,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn format_set(Json(req): Json<FormatSetReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::set_format(&req.path, &params, &req.sheet, &req.range, &req.style) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
