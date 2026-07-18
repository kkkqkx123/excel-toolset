// Format category tools: set style, merge cells.

use serde_json::Value;
use std::collections::HashMap;

use super::helpers::*;
use crate::server::{ToolDef, ToolHandler};
use excel_core::types::{SecurityParams, Style};

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_format_set",
            description: "Set cell formatting: fonts, colors, borders, alignment, number format.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("range", string_prop("Target range like A1:C10", true)),
                    ("style", string_prop("JSON style definition", true)),
                    (
                        "dry_run",
                        bool_prop("If true, simulate without writing", Some(false)),
                    ),
                ],
                vec!["path", "sheet", "range", "style"],
            ),
        },
        ToolDef {
            name: "excel_format_merge",
            description: "Merge a range of cells into one.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("range", string_prop("Range to merge like A1:C1", true)),
                    ("value", string_prop("Value for the merged cell", true)),
                    (
                        "dry_run",
                        bool_prop("If true, simulate without writing", Some(false)),
                    ),
                ],
                vec!["path", "sheet", "range", "value"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_format_set".into(), handle_set);
    handlers.insert("excel_format_merge".into(), handle_merge);
}

fn params(path: &str, dry_run: bool) -> SecurityParams {
    security_params(path, dry_run)
}

fn handle_set(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let range = get_string(&args, "range").unwrap_or_default();
    let style_str = get_string(&args, "style").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    let style: Style = match serde_json::from_str(&style_str) {
        Ok(s) => s,
        Err(e) => return format!("Error parsing style JSON: {e}"),
    };

    match excel_core::excel_write::set_format(
        &path,
        &params(&path, dry_run),
        &sheet,
        &range,
        &style,
    ) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_merge(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let range = get_string(&args, "range").unwrap_or_default();
    let value = get_string(&args, "value").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    match excel_core::excel_write::merge_cells(
        &path,
        &params(&path, dry_run),
        &sheet,
        &range,
        &value,
    ) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}
