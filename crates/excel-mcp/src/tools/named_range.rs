// Named range category tools.

use serde_json::Value;
use std::collections::HashMap;

use super::helpers::*;
use crate::server::{ToolDef, ToolHandler};

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_named_range_list",
            description: "List all named ranges in an Excel workbook.",
            input_schema: object_schema(
                vec![("path", string_prop("Path to the .xlsx file", true))],
                vec!["path"],
            ),
        },
        ToolDef {
            name: "excel_named_range_get",
            description: "Get the value of a named range.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("name", string_prop("Named range name", true)),
                ],
                vec!["path", "name"],
            ),
        },
        ToolDef {
            name: "excel_named_range_create",
            description: "Create a new named range.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("name", string_prop("Named range name", true)),
                    ("range", string_prop("Cell range like A1:C10", true)),
                    ("sheet", string_prop("Optional sheet scope", false)),
                    (
                        "dry_run",
                        bool_prop("If true, simulate without writing", Some(false)),
                    ),
                ],
                vec!["path", "name", "range"],
            ),
        },
        ToolDef {
            name: "excel_named_range_delete",
            description: "Delete a named range.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("name", string_prop("Named range name", true)),
                    (
                        "dry_run",
                        bool_prop("If true, simulate without writing", Some(false)),
                    ),
                ],
                vec!["path", "name"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_named_range_list".into(), handle_list);
    handlers.insert("excel_named_range_get".into(), handle_get);
    handlers.insert("excel_named_range_create".into(), handle_create);
    handlers.insert("excel_named_range_delete".into(), handle_delete);
}

fn handle_list(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    match excel_core::features::named_ranges::list_named_ranges(&path) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_get(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let name = get_string(&args, "name").unwrap_or_default();
    match excel_core::features::named_ranges::get_named_range_value(&path, &name) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_create(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let name = get_string(&args, "name").unwrap_or_default();
    let range = get_string(&args, "range").unwrap_or_default();
    let sheet = get_string(&args, "sheet");
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    match excel_core::features::named_ranges::create_named_range(
        &path,
        &name,
        &range,
        sheet.as_deref(),
        &security_params(&path, dry_run),
    ) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_delete(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let name = get_string(&args, "name").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    match excel_core::features::named_ranges::delete_named_range(
        &path,
        &name,
        &security_params(&path, dry_run),
    ) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}
