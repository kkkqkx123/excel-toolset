// Sheet category tools: list, add, delete, rename.

use std::collections::HashMap;
use serde_json::Value;

use crate::server::{ToolDef, ToolHandler};
use super::helpers::*;

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_sheet_list",
            description: "List all sheet names in an Excel workbook.",
            input_schema: object_schema(
                vec![("path", string_prop("Path to the .xlsx file", true))],
                vec!["path"],
            ),
        },
        ToolDef {
            name: "excel_sheet_add",
            description: "Add a new sheet to an Excel workbook.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("name", string_prop("Name of the new sheet", true)),
                ],
                vec!["path", "name"],
            ),
        },
        ToolDef {
            name: "excel_sheet_delete",
            description: "Delete a sheet from an Excel workbook.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("name", string_prop("Name of the sheet to delete", true)),
                ],
                vec!["path", "name"],
            ),
        },
        ToolDef {
            name: "excel_sheet_rename",
            description: "Rename a sheet in an Excel workbook.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("old", string_prop("Current sheet name", true)),
                    ("new", string_prop("New sheet name", true)),
                ],
                vec!["path", "old", "new"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_sheet_list".into(), handle_sheet_list);
    handlers.insert("excel_sheet_add".into(), handle_sheet_add);
    handlers.insert("excel_sheet_delete".into(), handle_sheet_delete);
    handlers.insert("excel_sheet_rename".into(), handle_sheet_rename);
}

fn handle_sheet_list(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    match excel_core::excel_read::list_sheets(&path) {
        Ok(sheets) => to_result_string(&sheets),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_sheet_add(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let name = get_string(&args, "name").unwrap_or_default();
    let params = security_params(&path, false);
    match excel_core::excel_write::add_sheet(&path, &params, &name) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_sheet_delete(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let name = get_string(&args, "name").unwrap_or_default();
    let params = security_params(&path, false);
    match excel_core::excel_write::delete_sheet(&path, &params, &name) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_sheet_rename(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let old = get_string(&args, "old").unwrap_or_default();
    let new = get_string(&args, "new").unwrap_or_default();
    let params = security_params(&path, false);
    match excel_core::excel_write::rename_sheet(&path, &params, &old, &new) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}
