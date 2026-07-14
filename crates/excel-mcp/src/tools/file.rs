// File category tools: create, info, backup.

use std::collections::HashMap;
use serde_json::Value;

use crate::server::{ToolDef, ToolHandler};
use super::helpers::*;

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_file_create",
            description: "Create a new Excel file (.xlsx) with an optional sheet name.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the new .xlsx file", true)),
                    ("sheet", string_prop("Name of the initial sheet (default: Sheet1)", false)),
                ],
                vec!["path"],
            ),
        },
        ToolDef {
            name: "excel_file_info",
            description: "Get information about an Excel file, including sheets, size, and SHA-256 hash.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                ],
                vec!["path"],
            ),
        },
        ToolDef {
            name: "excel_file_backup",
            description: "Create a timestamped backup of an Excel file.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                ],
                vec!["path"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_file_create".into(), handle_file_create);
    handlers.insert("excel_file_info".into(), handle_file_info);
    handlers.insert("excel_file_backup".into(), handle_file_backup);
}

fn handle_file_create(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_else(|| "Sheet1".into());

    match excel_core::excel_write::create_file(&path, &sheet) {
        Ok(info) => to_result_string(&info),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_file_info(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();

    match excel_core::excel_read::read_file_info(&path) {
        Ok(info) => to_result_string(&info),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_file_backup(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();

    match excel_core::security::create_backup(&path, "manual_backup") {
        Ok(info) => to_result_string(&info),
        Err(e) => format!("Error: {e}"),
    }
}
