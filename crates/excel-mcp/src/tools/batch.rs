// Batch category tools.

use serde_json::Value;
use std::collections::HashMap;

use super::helpers::*;
use crate::server::{ToolDef, ToolHandler};
use excel_core::types::BatchOperation;

pub fn tools() -> Vec<ToolDef> {
    vec![ToolDef {
        name: "excel_batch_modify",
        description: "Execute multiple operations atomically. Operations are a JSON string of BatchOperation array.",
        input_schema: object_schema(
            vec![
                ("path", string_prop("Path to the .xlsx file", true)),
                (
                    "operations",
                    string_prop("JSON string of BatchOperation array", true),
                ),
                (
                    "dry_run",
                    bool_prop("If true, simulate without writing", Some(false)),
                ),
            ],
            vec!["path", "operations"],
        ),
    }]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_batch_modify".into(), handle_modify);
}

fn handle_modify(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let ops_str = get_string(&args, "operations").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    let operations: Vec<BatchOperation> = match serde_json::from_str(&ops_str) {
        Ok(ops) => ops,
        Err(e) => return format!("Error parsing operations JSON: {e}"),
    };

    let params = security_params(&path, dry_run);

    match excel_core::excel_write::execute_batch_operations(&path, &params, &operations) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}
