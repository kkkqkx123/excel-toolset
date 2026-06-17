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

fn parse_rule_type(s: &str) -> conditional_format::ConditionalFormatType {
    match s.to_lowercase().as_str() {
        "cellvalue" | "cell_value" | "cell" => conditional_format::ConditionalFormatType::CellValue,
        "formula" => conditional_format::ConditionalFormatType::Formula,
        "aboveaverage" | "above_average" => conditional_format::ConditionalFormatType::AboveAverage,
        "top10" => conditional_format::ConditionalFormatType::Top10,
        "duplicate" => conditional_format::ConditionalFormatType::Duplicate,
        "textcontains" | "text_contains" => conditional_format::ConditionalFormatType::TextContains,
        "dateoccurring" | "date_occurring" => {
            conditional_format::ConditionalFormatType::DateOccurring
        }
        _ => conditional_format::ConditionalFormatType::CellValue,
    }
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
        rule_type: parse_rule_type(&req.rule_type),
        condition: req.condition,
        format: req.format,
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
