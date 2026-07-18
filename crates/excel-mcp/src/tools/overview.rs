// Overview tool: workbook overview with optional blueprint.

use serde_json::Value;
use std::collections::HashMap;

use super::helpers::*;
use crate::server::{ToolDef, ToolHandler};

pub fn tools() -> Vec<ToolDef> {
    vec![ToolDef {
        name: "excel_overview",
        description: "Get a comprehensive overview of an Excel workbook: sheets, data ranges, formulas, styles. Optionally generate a blueprint for LLM consumption.",
        input_schema: object_schema(
            vec![
                ("path", string_prop("Path to the .xlsx file", true)),
                (
                    "blueprint",
                    bool_prop(
                        "Generate a compact blueprint for LLM processing",
                        Some(false),
                    ),
                ),
            ],
            vec!["path"],
        ),
    }]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_overview".into(), handle_overview);
}

fn handle_overview(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let blueprint = get_bool(&args, "blueprint").unwrap_or(false);

    if blueprint {
        match excel_core::features::workbook_overview::get_workbook_blueprint(&path) {
            Ok(r) => to_result_string(&r),
            Err(e) => format!("Error: {e}"),
        }
    } else {
        match excel_core::features::workbook_overview::get_workbook_overview(&path) {
            Ok(r) => to_result_string(&r),
            Err(e) => format!("Error: {e}"),
        }
    }
}
