use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

use excel_core::features::formula_eval;
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

#[derive(Deserialize)]
pub struct FormulaEvalReq {
    pub path: String,
    pub sheet: String,
    pub cell: String,
    pub formula: String,
    #[serde(default = "default_true")]
    pub evaluate: bool,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct FormulaEvalBatchReq {
    pub path: String,
    pub sheet: String,
    pub formulas: Vec<FormulaItem>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct FormulaItem {
    pub cell: String,
    pub formula: String,
}

fn default_true() -> bool {
    true
}

pub async fn formula_fill(Json(req): Json<FormulaFillReq>) -> impl IntoResponse {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: !req.dry_run,
        file_path: req.path.clone(),
    };
    match formula_ops::fill_formula(
        &req.path,
        &req.sheet,
        &req.source,
        &req.target_range,
        &params,
    ) {
        Ok(result) => {
            let body = serde_json::to_string(&ApiResponse::ok(Some(result))).unwrap_or_default();
            Ok((StatusCode::OK, [("content-type", "application/json")], body))
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

pub async fn formula_evaluate(Json(req): Json<FormulaEvalReq>) -> impl IntoResponse {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: !req.dry_run,
        file_path: req.path.clone(),
    };
    match formula_eval::set_formula_with_eval(
        &req.path,
        &req.sheet,
        &req.cell,
        &req.formula,
        req.evaluate,
        &params,
    ) {
        Ok(result) => {
            let body = serde_json::to_string(&ApiResponse::ok(Some(result))).unwrap_or_default();
            Ok((StatusCode::OK, [("content-type", "application/json")], body))
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

pub async fn formula_evaluate_batch(Json(req): Json<FormulaEvalBatchReq>) -> impl IntoResponse {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: !req.dry_run,
        file_path: req.path.clone(),
    };

    let mut results = Vec::new();
    for item in &req.formulas {
        match formula_eval::set_formula_with_eval(
            &req.path,
            &req.sheet,
            &item.cell,
            &item.formula,
            true,
            &params,
        ) {
            Ok(r) => results.push(serde_json::json!({"cell": item.cell, "result": r})),
            Err(e) => results.push(serde_json::json!({"cell": item.cell, "error": e.to_string()})),
        }
    }

    let body = serde_json::to_string(&ApiResponse::ok(Some(results))).unwrap_or_default();
    Ok::<_, (StatusCode, [(&str, &str); 1], String)>((
        StatusCode::OK,
        [("content-type", "application/json")],
        body,
    ))
}
