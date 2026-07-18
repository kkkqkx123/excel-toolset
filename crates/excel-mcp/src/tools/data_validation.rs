// Data validation category tools.

use serde_json::Value;
use std::collections::HashMap;

use super::helpers::*;
use crate::server::{ToolDef, ToolHandler};
use excel_core::types::{DataValidationConfig, SecurityParams};

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_data_validation_add",
            description: "Add data validation to cells. Config is a JSON DataValidationConfig object.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("config", string_prop("JSON DataValidationConfig", true)),
                    (
                        "dry_run",
                        bool_prop("If true, simulate without writing", Some(false)),
                    ),
                ],
                vec!["path", "sheet", "config"],
            ),
        },
        ToolDef {
            name: "excel_data_validation_remove",
            description: "Remove data validation from a range of cells.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    (
                        "range",
                        string_prop("Range to remove validation from", true),
                    ),
                    (
                        "dry_run",
                        bool_prop("If true, simulate without writing", Some(false)),
                    ),
                ],
                vec!["path", "sheet", "range"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_data_validation_add".into(), handle_add);
    handlers.insert("excel_data_validation_remove".into(), handle_remove);
}

fn params(path: &str, dry_run: bool) -> SecurityParams {
    security_params(path, dry_run)
}

fn handle_add(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let config_str = get_string(&args, "config").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    let config: DataValidationConfig = match serde_json::from_str(&config_str) {
        Ok(c) => c,
        Err(e) => return format!("Error parsing config JSON: {e}"),
    };

    match excel_core::excel_write::add_data_validation(
        &path,
        &params(&path, dry_run),
        &sheet,
        &config,
    ) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_remove(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let range = get_string(&args, "range").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    match excel_core::excel_write::remove_data_validation(
        &path,
        &params(&path, dry_run),
        &sheet,
        &range,
    ) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}
