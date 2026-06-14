use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct BatchModifyReq {
    pub path: String,
    pub operations: Vec<BatchOperation>,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default = "default_json_format")]
    pub format: String,
}

fn default_json_format() -> String {
    "json".into()
}

pub async fn batch_modify(Json(req): Json<BatchModifyReq>) -> impl IntoResponse {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    let mut result =
        match excel_write::execute_batch_operations(&req.path, &params, &req.operations) {
            Ok(r) => r,
            Err(e) => {
                let body = serde_json::to_string(&ApiResponse::<()>::err(e)).unwrap_or_default();
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [("content-type", "application/json")],
                    body,
                ));
            }
        };
    if let Some(ref backup) = result.backup_info
        && let Ok(diff) = excel_diff::diff_files(&backup.backup_path, &req.path)
    {
        result.diff = Some(diff);
    }
    if req.format == "text" {
        let text = if let Some(ref diff) = result.diff {
            excel_diff::semantic::to_natural_text(
                diff,
                None,
                excel_diff::semantic::Verbosity::Detail,
            )
        } else {
            "Batch modify completed (no changes detected).".to_string()
        };
        Ok((
            StatusCode::OK,
            [("content-type", "text/plain; charset=utf-8")],
            text,
        ))
    } else {
        Ok((
            StatusCode::OK,
            [("content-type", "application/json")],
            serde_json::to_string(&ApiResponse::ok(Some(result))).unwrap_or_default(),
        ))
    }
}