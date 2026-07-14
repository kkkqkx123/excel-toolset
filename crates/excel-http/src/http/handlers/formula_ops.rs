use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

use excel_core::features::formula_ops;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct FormulaFillReq {
    pub path: String,
    pub sheet: String,
    pub source: String,
    pub target_range: String,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn formula_fill(Json(req): Json<FormulaFillReq>) -> impl IntoResponse {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: !req.dry_run,
        file_path: req.path.clone(),
    };
    match formula_ops::fill_formula(&req.path, &req.sheet, &req.source, &req.target_range, &params)
    {
        Ok(result) => {
            let body = serde_json::to_string(&ApiResponse::ok(Some(result))).unwrap_or_default();
            Ok((
                StatusCode::OK,
                [("content-type", "application/json")],
                body,
            ))
        }
        Err(e) => {
            let body = serde_json::to_string(&ApiResponse::<()>::err(e)).unwrap_or_default();
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "application/json")],
                body,
            ))
        }
    }
}
