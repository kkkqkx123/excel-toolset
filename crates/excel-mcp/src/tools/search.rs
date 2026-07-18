// Search category tools.

use serde_json::Value;
use std::collections::HashMap;

use super::helpers::*;
use crate::server::{ToolDef, ToolHandler};
use excel_core::features::search::{MatchType, SearchQuery, SearchType};

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_search_workbook",
            description: "Search for text across all sheets or specified sheets in a workbook.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("pattern", string_prop("Search pattern", true)),
                    (
                        "match_type",
                        enum_prop("Match type", &["contains", "exact", "regex"]),
                    ),
                    (
                        "search_type",
                        enum_prop(
                            "Search in values, formulas, or both",
                            &["values", "formulas", "both"],
                        ),
                    ),
                    (
                        "case_sensitive",
                        bool_prop("Case sensitive search", Some(false)),
                    ),
                    (
                        "sheets",
                        string_array_prop("Optional: specific sheets to search"),
                    ),
                ],
                vec!["path", "pattern"],
            ),
        },
        ToolDef {
            name: "excel_search_sheet",
            description: "Search for text in a specific sheet.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("pattern", string_prop("Search pattern", true)),
                    (
                        "match_type",
                        enum_prop("Match type", &["contains", "exact", "regex"]),
                    ),
                    (
                        "search_type",
                        enum_prop(
                            "Search in values, formulas, or both",
                            &["values", "formulas", "both"],
                        ),
                    ),
                    (
                        "case_sensitive",
                        bool_prop("Case sensitive search", Some(false)),
                    ),
                ],
                vec!["path", "sheet", "pattern"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_search_workbook".into(), handle_workbook);
    handlers.insert("excel_search_sheet".into(), handle_sheet);
}

fn parse_search_type(s: &str) -> SearchType {
    match s {
        "values" => SearchType::Value,
        "formulas" => SearchType::Formula,
        _ => SearchType::Both,
    }
}

fn parse_match_type(s: &str) -> MatchType {
    match s {
        "exact" => MatchType::Exact,
        "regex" => MatchType::Regex,
        _ => MatchType::Contains,
    }
}

fn handle_workbook(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let pattern = get_string(&args, "pattern").unwrap_or_default();
    let match_type = get_string(&args, "match_type").unwrap_or_else(|| "contains".into());
    let search_type = get_string(&args, "search_type").unwrap_or_else(|| "both".into());
    let case_sensitive = get_bool(&args, "case_sensitive").unwrap_or(false);
    let sheets = get_string_array(&args, "sheets");

    let query = SearchQuery {
        pattern,
        search_type: parse_search_type(&search_type),
        match_type: parse_match_type(&match_type),
        case_sensitive,
        sheets,
    };

    match excel_core::features::search::search_workbook(&path, &query) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_sheet(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let pattern = get_string(&args, "pattern").unwrap_or_default();
    let match_type = get_string(&args, "match_type").unwrap_or_else(|| "contains".into());
    let search_type = get_string(&args, "search_type").unwrap_or_else(|| "both".into());
    let case_sensitive = get_bool(&args, "case_sensitive").unwrap_or(false);

    let query = SearchQuery {
        pattern,
        search_type: parse_search_type(&search_type),
        match_type: parse_match_type(&match_type),
        case_sensitive,
        sheets: None,
    };

    match excel_core::features::search::search_sheet(&path, &sheet, &query) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}
