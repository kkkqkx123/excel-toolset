use axum::Json;
use serde::Deserialize;

use excel_core::features::conditional_format;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct AddConditionalFormatReq {
    pub path: String,
    pub sheet: String,
    pub range: String,
    pub rule_type: String,
    pub condition: String,
    pub format: Option<Style>,
    /// JSON config for DataBar, ColorScale, IconSet types.
    pub config: Option<conditional_format::ConditionalFormatConfig>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct RemoveConditionalFormatReq {
    pub path: String,
    pub sheet: String,
    pub range: String,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn add_conditional_format(
    Json(req): Json<AddConditionalFormatReq>,
) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };

    let rule = conditional_format::ConditionalFormatRule {
        rule_type: conditional_format::parse_rule_type(&req.rule_type),
        condition: req.condition,
        format: req.format,
        config: req.config,
    };

    match conditional_format::add_conditional_format(
        &req.path, &req.sheet, &req.range, &rule, &params,
    ) {
        Ok(result) => Json(ApiResponse::ok(Some(result))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn remove_conditional_format(
    Json(req): Json<RemoveConditionalFormatReq>,
) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };

    match conditional_format::remove_conditional_format(&req.path, &req.sheet, &req.range, &params)
    {
        Ok(result) => Json(ApiResponse::ok(Some(result))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
