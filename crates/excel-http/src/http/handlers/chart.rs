use axum::Json;
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::types::*;
use excel_core::utils::helpers;

#[derive(Deserialize)]
pub struct ChartCreateReq {
    pub path: String,
    pub sheet: String,
    pub range: String,
    pub chart_type: String,
    pub title: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn chart_create(Json(req): Json<ChartCreateReq>) -> Json<ApiResponse<WriteResult>> {
    let ct = match helpers::chart_type_from_str(&req.chart_type) {
        Ok(t) => t,
        Err(e) => return Json(ApiResponse::err(e)),
    };
    let (r1, c1, _, _) = match excel_core::utils::cell_ref::parse_range(&req.range) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::err(e)),
    };
    let config = ChartConfig {
        chart_type: ct,
        title: req.title,
        categories_range: req.range.clone(),
        values_range: req.range,
        sheet: req.sheet,
        row: r1,
        col: c1,
    };
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::add_chart(&req.path, &params, &config) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
