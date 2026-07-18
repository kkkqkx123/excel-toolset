// Freeze panes category tools: set, clear.

use serde_json::Value;
use std::collections::HashMap;

use super::helpers::*;
use crate::server::{ToolDef, ToolHandler};

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_freeze_panes_set",
            description: "Set freeze panes on a worksheet. Freezes specified rows from top and/or columns from left.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Target sheet name", true)),
                    (
                        "rows",
                        int_prop("Number of rows to freeze from top (0 = no row freeze)"),
                    ),
                    (
                        "cols",
                        int_prop("Number of columns to freeze from left (0 = no column freeze)"),
                    ),
                ],
                vec!["path", "sheet"],
            ),
        },
        ToolDef {
            name: "excel_freeze_panes_clear",
            description: "Clear freeze panes from a worksheet.",
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
    handlers.insert("excel_freeze_panes_set".into(), handle_freeze_panes_set);
    handlers.insert("excel_freeze_panes_clear".into(), handle_freeze_panes_clear);
}

fn handle_freeze_panes_set(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let rows = get_u32(&args, "rows").unwrap_or(0);
    let cols = get_u32(&args, "cols").unwrap_or(0) as u16;
    let config = excel_core::types::FreezePanesConfig { sheet, rows, cols };
    let params = security_params(&path, false);
    match excel_core::excel_write::set_freeze_panes(&path, &params, &config) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_freeze_panes_clear(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let params = security_params(&path, false);
    match excel_core::excel_write::clear_freeze_panes(&path, &params, &sheet) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}
