// Slicer category tools.

use serde_json::Value;
use std::collections::HashMap;

use super::helpers::*;
use crate::server::{ToolDef, ToolHandler};
use excel_core::types::SlicerConfig;

pub fn tools() -> Vec<ToolDef> {
    vec![ToolDef {
        name: "excel_slicer_create",
        description: "Create a slicer for a pivot table field. Config is a JSON SlicerConfig object with name, pivot_table_name, source_range, field_column, target_sheet, and position.",
        input_schema: object_schema(
            vec![
                ("path", string_prop("Path to the .xlsx file", true)),
                ("config", string_prop("JSON SlicerConfig", true)),
                (
                    "dry_run",
                    bool_prop("If true, simulate without writing", Some(false)),
                ),
            ],
            vec!["path", "config"],
        ),
    }]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_slicer_create".into(), handle_create);
}

fn handle_create(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let config_str = get_string(&args, "config").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    let config: SlicerConfig = match serde_json::from_str(&config_str) {
        Ok(c) => c,
        Err(e) => return format!("Error parsing config JSON: {e}"),
    };

    let params = security_params(&path, dry_run);

    match excel_core::excel_write::create_slicer(&path, &params, &config) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}
