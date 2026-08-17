use axum::Json;
use serde::Deserialize;

use crate::http::response::ApiJson;
use excel_core::features::formula_analysis;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct TraceDependenciesReq {
    pub path: String,
    pub sheet: String,
    pub cell: String,
}

#[derive(Deserialize)]
pub struct ExplainFormulaReq {
    pub path: String,
    pub sheet: String,
    pub cell: String,
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_language() -> String {
    "en".to_string()
}

pub async fn trace_dependencies(
    Json(req): Json<TraceDependenciesReq>,
) -> ApiJson<formula_analysis::DependencyTrace> {
    match formula_analysis::trace_dependencies(&req.path, &req.sheet, &req.cell) {
        Ok(trace) => ApiJson(ApiResponse::ok(Some(trace))),
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}

pub async fn explain_formula(
    Json(req): Json<ExplainFormulaReq>,
) -> ApiJson<formula_analysis::FormulaExplanation> {
    match formula_analysis::explain_formula(&req.path, &req.sheet, &req.cell, &req.language) {
        Ok(explanation) => ApiJson(ApiResponse::ok(Some(explanation))),
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}

pub async fn explain_formula_logic(
    Json(req): Json<ExplainFormulaReq>,
) -> ApiJson<formula_analysis::FormulaLogicExplanation> {
    match formula_analysis::explain_formula_logic(&req.path, &req.sheet, &req.cell, &req.language) {
        Ok(explanation) => ApiJson(ApiResponse::ok(Some(explanation))),
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}
