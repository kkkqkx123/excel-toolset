// Sheet protection category tools: protect, unprotect, is_protected.

use serde_json::Value;
use std::collections::HashMap;

use super::helpers::*;
use crate::server::{ToolDef, ToolHandler};

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_sheet_protect",
            description: "Protect a worksheet from modification. Optionally set a password and specify which operations are allowed. If options are not provided, defaults to allowing only selecting locked/unlocked cells.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Target sheet name", true)),
                    (
                        "password",
                        string_prop("Optional password for protection", false),
                    ),
                    (
                        "options",
                        string_prop(
                            "Optional JSON object with protection options keys: select_locked_cells, select_unlocked_cells, format_cells, format_columns, format_rows, insert_rows, insert_columns, insert_links, delete_rows, delete_columns, sort, auto_filter, pivot_tables, edit_scenarios, edit_objects, contents",
                            false,
                        ),
                    ),
                ],
                vec!["path", "sheet"],
            ),
        },
        ToolDef {
            name: "excel_sheet_unprotect",
            description: "Remove protection from a worksheet.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Target sheet name", true)),
                ],
                vec!["path", "sheet"],
            ),
        },
        ToolDef {
            name: "excel_sheet_is_protected",
            description: "Check if a worksheet is protected. Returns true or false.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Target sheet name", true)),
                ],
                vec!["path", "sheet"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_sheet_protect".into(), handle_sheet_protect);
    handlers.insert("excel_sheet_unprotect".into(), handle_sheet_unprotect);
    handlers.insert("excel_sheet_is_protected".into(), handle_sheet_is_protected);
}

fn handle_sheet_protect(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let password = get_string(&args, "password");

    let options = if let Some(opts_str) = get_string(&args, "options") {
        match serde_json::from_str::<excel_core::types::ProtectionOptions>(&opts_str) {
            Ok(opts) => opts,
            Err(e) => return format!("Error: Invalid options JSON: {e}"),
        }
    } else {
        excel_core::types::ProtectionOptions::default()
    };

    let config = excel_core::types::SheetProtectionConfig {
        sheet,
        password,
        options,
    };
    let params = security_params(&path, false);
    match excel_core::excel_write::protect_sheet(&path, &params, &config) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_sheet_unprotect(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let params = security_params(&path, false);
    match excel_core::excel_write::unprotect_sheet(&path, &params, &sheet) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_sheet_is_protected(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    match excel_core::excel_write::is_sheet_protected(&path, &sheet) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}
