use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

use excel_core::types::*;
use excel_diff::diff_files;
use excel_diff::diff_formula_dependencies;
use excel_diff::diff_range;
use excel_diff::diff_sheets;
use excel_diff::diff_with_semantic;
use excel_diff::semantic::{self, Verbosity};
use excel_diff::summarize;

#[derive(Deserialize)]
pub struct DiffFileReq {
    pub old_path: String,
    pub new_path: String,
    pub sheet: Option<String>,
    #[serde(default = "default_json_format")]
    pub format: String,
    #[serde(default)]
    pub semantic: bool,
}

fn default_json_format() -> String {
    "json".into()
}

#[derive(Deserialize)]
pub struct DiffRangeReq {
    pub old_path: String,
    pub new_path: String,
    pub sheet: String,
    pub range: String,
    #[serde(default = "default_json_format")]
    pub format: String,
    #[serde(default)]
    pub semantic: bool,
}

#[derive(Deserialize)]
pub struct SemanticDiffReq {
    pub old_path: String,
    pub new_path: String,
}

#[derive(Deserialize)]
pub struct FormulaDepsReq {
    pub old_path: String,
    pub new_path: String,
    pub sheet: String,
}

pub async fn diff_file(Json(req): Json<DiffFileReq>) -> impl IntoResponse {
    if req.semantic {
        match diff_with_semantic(&req.old_path, &req.new_path) {
            Ok(sd) => match serde_json::to_value(sd) {
                Ok(val) => {
                    let body =
                        serde_json::to_string(&ApiResponse::ok(Some(val))).unwrap_or_default();
                    return Ok((StatusCode::OK, [("content-type", "application/json")], body));
                }
                Err(e) => {
                    let body = serde_json::to_string(&ApiResponse::<()>::err(
                        AppError::Serialize(e.to_string()),
                    ))
                    .unwrap_or_default();
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        [("content-type", "application/json")],
                        body,
                    ));
                }
            },
            Err(e) => {
                let body = serde_json::to_string(&ApiResponse::<()>::err(e)).unwrap_or_default();
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [("content-type", "application/json")],
                    body,
                ));
            }
        }
    }

    let diff_result = if let Some(ref sheet_name) = req.sheet {
        diff_sheets(&req.old_path, &req.new_path, sheet_name).map(|sd| {
            let summary = summarize::summarize(std::slice::from_ref(&sd));
            FileDiff {
                file_hash_match: false,
                sheet_diffs: vec![sd],
                summary,
            }
        })
    } else {
        diff_files(&req.old_path, &req.new_path)
    };

    match diff_result {
        Ok(diff) => {
            if req.format == "text" {
                let text = semantic::to_natural_text(&diff, None, Verbosity::Detail);
                Ok((
                    StatusCode::OK,
                    [("content-type", "text/plain; charset=utf-8")],
                    text,
                ))
            } else {
                match serde_json::to_value(diff) {
                    Ok(val) => {
                        let body =
                            serde_json::to_string(&ApiResponse::ok(Some(val))).unwrap_or_default();
                        Ok((StatusCode::OK, [("content-type", "application/json")], body))
                    }
                    Err(e) => {
                        let body = serde_json::to_string(&ApiResponse::<()>::err(
                            AppError::Serialize(e.to_string()),
                        ))
                        .unwrap_or_default();
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            [("content-type", "application/json")],
                            body,
                        ))
                    }
                }
            }
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

pub async fn handle_diff_range(Json(req): Json<DiffRangeReq>) -> impl IntoResponse {
    if req.semantic {
        let range_diff = match diff_range(&req.old_path, &req.new_path, &req.sheet, &req.range) {
            Ok(d) => d,
            Err(e) => {
                let body =
                    serde_json::to_string(&ApiResponse::<()>::err(e)).unwrap_or_default();
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [("content-type", "application/json")],
                    body,
                ));
            }
        };
        let sd = SheetDiff {
            sheet_name: req.sheet.clone(),
            row_count_diff: 0,
            col_count_diff: 0,
            cell_diffs: range_diff.cell_diffs,
        };
        let summary = summarize::summarize(std::slice::from_ref(&sd));
        let fd = FileDiff {
            file_hash_match: false,
            sheet_diffs: vec![sd],
            summary,
        };
        let text = semantic::to_natural_text(&fd, None, Verbosity::Detail);
        match serde_json::to_value(serde_json::json!({"raw_text": text})) {
            Ok(val) => {
                let body =
                    serde_json::to_string(&ApiResponse::ok(Some(val))).unwrap_or_default();
                Ok((StatusCode::OK, [("content-type", "application/json")], body))
            }
            Err(e) => {
                let body = serde_json::to_string(&ApiResponse::<()>::err(
                    AppError::Serialize(e.to_string()),
                ))
                .unwrap_or_default();
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [("content-type", "application/json")],
                    body,
                ))
            }
        }
    } else {
        let diff_result = diff_range(&req.old_path, &req.new_path, &req.sheet, &req.range);
        match diff_result {
            Ok(data) => {
                if req.format == "text" {
                    let sd = SheetDiff {
                        sheet_name: req.sheet.clone(),
                        row_count_diff: 0,
                        col_count_diff: 0,
                        cell_diffs: data.cell_diffs,
                    };
                    let summary = summarize::summarize(std::slice::from_ref(&sd));
                    let fd = FileDiff {
                        file_hash_match: false,
                        sheet_diffs: vec![sd],
                        summary,
                    };
                    let text = semantic::to_natural_text(&fd, None, Verbosity::Detail);
                    Ok((
                        StatusCode::OK,
                        [("content-type", "text/plain; charset=utf-8")],
                        text,
                    ))
                } else {
                    match serde_json::to_value(data) {
                        Ok(val) => {
                            let body =
                                serde_json::to_string(&ApiResponse::ok(Some(val)))
                                    .unwrap_or_default();
                            Ok((StatusCode::OK, [("content-type", "application/json")], body))
                        }
                        Err(e) => {
                            let body = serde_json::to_string(&ApiResponse::<()>::err(
                                AppError::Serialize(e.to_string()),
                            ))
                            .unwrap_or_default();
                            Err((
                                StatusCode::INTERNAL_SERVER_ERROR,
                                [("content-type", "application/json")],
                                body,
                            ))
                        }
                    }
                }
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
}

pub async fn diff_semantic(Json(req): Json<SemanticDiffReq>) -> impl IntoResponse {
    match diff_with_semantic(&req.old_path, &req.new_path) {
        Ok(sd) => match serde_json::to_value(sd) {
            Ok(val) => {
                let body =
                    serde_json::to_string(&ApiResponse::ok(Some(val))).unwrap_or_default();
                Ok((StatusCode::OK, [("content-type", "application/json")], body))
            }
            Err(e) => {
                let body = serde_json::to_string(&ApiResponse::<()>::err(
                    AppError::Serialize(e.to_string()),
                ))
                .unwrap_or_default();
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [("content-type", "application/json")],
                    body,
                ))
            }
        },
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

pub async fn diff_formula_dependencies_handler(
    Json(req): Json<FormulaDepsReq>,
) -> impl IntoResponse {
    match diff_formula_dependencies(&req.old_path, &req.new_path, &req.sheet) {
        Ok(deps) => match serde_json::to_value(deps) {
            Ok(val) => {
                let body =
                    serde_json::to_string(&ApiResponse::ok(Some(val))).unwrap_or_default();
                Ok((StatusCode::OK, [("content-type", "application/json")], body))
            }
            Err(e) => {
                let body = serde_json::to_string(&ApiResponse::<()>::err(
                    AppError::Serialize(e.to_string()),
                ))
                .unwrap_or_default();
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [("content-type", "application/json")],
                    body,
                ))
            }
        },
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
