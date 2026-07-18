// Sparkline category tools.

use serde_json::Value;
use std::collections::HashMap;

use super::helpers::*;
use crate::server::{ToolDef, ToolHandler};
use excel_core::types::{SecurityParams, SparklineConfig};

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_sparkline_add",
            description: "Add a sparkline (mini chart in a cell). Config is a JSON SparklineConfig object.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    (
                        "config",
                        string_prop(
                            "JSON SparklineConfig: {sheet, source_range, target_cell, sparkline_type?, style?}",
                            true,
                        ),
                    ),
                    (
                        "dry_run",
                        bool_prop("If true, simulate without writing", Some(false)),
                    ),
                ],
                vec!["path", "config"],
            ),
        },
        ToolDef {
            name: "excel_sparkline_remove",
            description: "Remove a sparkline from a cell.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    (
                        "target_cell",
                        string_prop("Cell containing the sparkline in row,col format", true),
                    ),
                    (
                        "dry_run",
                        bool_prop("If true, simulate without writing", Some(false)),
                    ),
                ],
                vec!["path", "sheet", "target_cell"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_sparkline_add".into(), handle_add);
    handlers.insert("excel_sparkline_remove".into(), handle_remove);
}

fn params(path: &str, dry_run: bool) -> SecurityParams {
    security_params(path, dry_run)
}

fn handle_add(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let config_str = get_string(&args, "config").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    let config: SparklineConfig = match serde_json::from_str(&config_str) {
        Ok(c) => c,
        Err(e) => return format!("Error parsing config JSON: {e}"),
    };

    match excel_core::excel_write::add_sparkline(&path, &params(&path, dry_run), &config) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_remove(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let target_cell = get_string(&args, "target_cell").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    // Parse "row,col" format
    let parts: Vec<&str> = target_cell.split(',').collect();
    if parts.len() != 2 {
        return format!(
            "Error: target_cell must be in 'row,col' format (0-indexed), got '{target_cell}'"
        );
    }
    let row: u32 = match parts[0].trim().parse() {
        Ok(r) => r,
        Err(e) => return format!("Error parsing row: {e}"),
    };
    let col: u16 = match parts[1].trim().parse() {
        Ok(c) => c,
        Err(e) => return format!("Error parsing col: {e}"),
    };

    match excel_core::excel_write::remove_sparkline(
        &path,
        &params(&path, dry_run),
        &sheet,
        row,
        col,
    ) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}
