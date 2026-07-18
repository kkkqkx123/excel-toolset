// AutoFilter category tools: set, remove, get.

use serde_json::Value;
use std::collections::HashMap;

use super::helpers::*;
use crate::server::{ToolDef, ToolHandler};

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_auto_filter_set",
            description: "Set an autofilter range on a worksheet. Adds dropdown arrows to column headers for filtering. The range must include the header row (e.g. 'A1:D100').",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Target sheet name", true)),
                    (
                        "range",
                        string_prop(
                            "Autofilter range including header row, e.g. 'A1:D100'",
                            true,
                        ),
                    ),
                ],
                vec!["path", "sheet", "range"],
            ),
        },
        ToolDef {
            name: "excel_auto_filter_remove",
            description: "Remove the autofilter from a worksheet.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Target sheet name", true)),
                ],
                vec!["path", "sheet"],
            ),
        },
        ToolDef {
            name: "excel_auto_filter_get",
            description: "Get the current autofilter state of a worksheet. Returns whether autofilter is enabled and its range.",
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
    handlers.insert("excel_auto_filter_set".into(), handle_auto_filter_set);
    handlers.insert("excel_auto_filter_remove".into(), handle_auto_filter_remove);
    handlers.insert("excel_auto_filter_get".into(), handle_auto_filter_get);
}

fn handle_auto_filter_set(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let range = get_string(&args, "range").unwrap_or_default();
    let config = excel_core::types::AutoFilterConfig { sheet, range };
    let params = security_params(&path, false);
    match excel_core::excel_write::set_auto_filter(&path, &params, &config) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_auto_filter_remove(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let params = security_params(&path, false);
    match excel_core::excel_write::remove_auto_filter(&path, &params, &sheet) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_auto_filter_get(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    match excel_core::excel_write::get_auto_filter(&path, &sheet) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}
