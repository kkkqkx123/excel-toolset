// Table (ListObject) category tools.

use serde_json::Value;
use std::collections::HashMap;

use super::helpers::*;
use crate::server::{ToolDef, ToolHandler};
use excel_core::types::{SecurityParams, TableConfig};

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_table_create",
            description: "Create a formatted Excel table (ListObject) from a data range. Config is a JSON TableConfig.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    (
                        "config",
                        string_prop("JSON TableConfig: {sheet, range, name?, style?}", true),
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
            name: "excel_table_remove",
            description: "Remove a table from an Excel workbook. The data remains, only the table formatting is removed.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("name", string_prop("Table name to remove", true)),
                    (
                        "dry_run",
                        bool_prop("If true, simulate without writing", Some(false)),
                    ),
                ],
                vec!["path", "name"],
            ),
        },
        ToolDef {
            name: "excel_table_list",
            description: "List all tables in an Excel workbook.",
            input_schema: object_schema(
                vec![("path", string_prop("Path to the .xlsx file", true))],
                vec!["path"],
            ),
        },
        ToolDef {
            name: "excel_table_get",
            description: "Get details of a specific table by name.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("name", string_prop("Table name", true)),
                ],
                vec!["path", "name"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_table_create".into(), handle_create);
    handlers.insert("excel_table_remove".into(), handle_remove);
    handlers.insert("excel_table_list".into(), handle_list);
    handlers.insert("excel_table_get".into(), handle_get);
}

fn params(path: &str, dry_run: bool) -> SecurityParams {
    security_params(path, dry_run)
}

fn handle_create(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let config_str = get_string(&args, "config").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    let config: TableConfig = match serde_json::from_str(&config_str) {
        Ok(c) => c,
        Err(e) => return format!("Error parsing config JSON: {e}"),
    };

    match excel_core::excel_write::create_table(&path, &params(&path, dry_run), &config) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_remove(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let name = get_string(&args, "name").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    match excel_core::excel_write::remove_table(&path, &params(&path, dry_run), &name) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_list(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    match excel_core::excel_write::list_tables(&path) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_get(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let name = get_string(&args, "name").unwrap_or_default();
    match excel_core::excel_write::get_table(&path, &name) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}
