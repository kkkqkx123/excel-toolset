// History tool: operation history for a workbook.

use serde_json::Value;
use std::collections::HashMap;

use super::helpers::*;
use crate::server::{ToolDef, ToolHandler};

pub fn tools() -> Vec<ToolDef> {
    vec![ToolDef {
        name: "excel_history",
        description: "Get the operation history (backup/change log) for an Excel workbook.",
        input_schema: object_schema(
            vec![("path", string_prop("Path to the .xlsx file", true))],
            vec!["path"],
        ),
    }]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_history".into(), handle_history);
}

fn handle_history(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();

    match excel_core::features::workbook_overview::list_workbook_history(&path) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}
