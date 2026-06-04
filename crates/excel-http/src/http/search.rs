use axum::Json;
use serde::Deserialize;

use excel_core::search::{self, MatchType, SearchQuery, SearchType};
use excel_core::types::*;

#[derive(Deserialize)]
pub struct SearchWorkbookReq {
    pub path: String,
    pub pattern: String,
    #[serde(default = "default_search_type")]
    pub search_type: String,
    #[serde(default = "default_match_type")]
    pub match_type: String,
    #[serde(default)]
    pub case_sensitive: bool,
    pub sheets: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct SearchSheetReq {
    pub path: String,
    pub sheet: String,
    pub pattern: String,
    #[serde(default = "default_search_type")]
    pub search_type: String,
    #[serde(default = "default_match_type")]
    pub match_type: String,
    #[serde(default)]
    pub case_sensitive: bool,
}

fn default_search_type() -> String {
    "both".to_string()
}

fn default_match_type() -> String {
    "contains".to_string()
}

fn parse_search_type(s: &str) -> SearchType {
    match s.to_lowercase().as_str() {
        "value" => SearchType::Value,
        "formula" => SearchType::Formula,
        "both" => SearchType::Both,
        _ => SearchType::Both,
    }
}

fn parse_match_type(s: &str) -> MatchType {
    match s.to_lowercase().as_str() {
        "exact" => MatchType::Exact,
        "contains" => MatchType::Contains,
        "regex" => MatchType::Regex,
        _ => MatchType::Contains,
    }
}

pub async fn search_workbook(
    Json(req): Json<SearchWorkbookReq>,
) -> Json<ApiResponse<search::SearchResults>> {
    let query = SearchQuery {
        pattern: req.pattern,
        search_type: parse_search_type(&req.search_type),
        match_type: parse_match_type(&req.match_type),
        case_sensitive: req.case_sensitive,
        sheets: req.sheets,
    };

    match search::search_workbook(&req.path, &query) {
        Ok(results) => Json(ApiResponse::ok(Some(results))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn search_sheet(
    Json(req): Json<SearchSheetReq>,
) -> Json<ApiResponse<search::SearchResults>> {
    let query = SearchQuery {
        pattern: req.pattern,
        search_type: parse_search_type(&req.search_type),
        match_type: parse_match_type(&req.match_type),
        case_sensitive: req.case_sensitive,
        sheets: Some(vec![req.sheet.clone()]),
    };

    match search::search_sheet(&req.path, &req.sheet, &query) {
        Ok(results) => Json(ApiResponse::ok(Some(results))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
