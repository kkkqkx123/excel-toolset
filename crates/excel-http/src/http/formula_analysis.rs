use axum::Json;
use serde::Deserialize;

use excel_core::formula_analysis;
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
) -> Json<ApiResponse<formula_analysis::DependencyTrace>> {
    match formula_analysis::trace_dependencies(&req.path, &req.sheet, &req.cell) {
        Ok(trace) => Json(ApiResponse::ok(Some(trace))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn explain_formula(
    Json(req): Json<ExplainFormulaReq>,
) -> Json<ApiResponse<formula_analysis::FormulaExplanation>> {
    match formula_analysis::explain_formula(&req.path, &req.sheet, &req.cell, &req.language) {
        Ok(explanation) => Json(ApiResponse::ok(Some(explanation))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn explain_formula_logic(
    Json(req): Json<ExplainFormulaReq>,
) -> Json<ApiResponse<formula_analysis::FormulaLogicExplanation>> {
    match formula_analysis::explain_formula_logic(&req.path, &req.sheet, &req.cell, &req.language) {
        Ok(explanation) => Json(ApiResponse::ok(Some(explanation))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
