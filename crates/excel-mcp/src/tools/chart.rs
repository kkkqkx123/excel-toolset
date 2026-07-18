// Chart category tools: create chart.

use serde_json::Value;
use std::collections::HashMap;

use super::helpers::*;
use crate::server::{ToolDef, ToolHandler};
use excel_core::types::ChartConfig;

pub fn tools() -> Vec<ToolDef> {
    vec![ToolDef {
        name: "excel_chart_create",
        description: "Create a chart from a data range. Config is a JSON object with sheet, chart_type, categories_range, values_range, title, row, col, etc.",
        input_schema: object_schema(
            vec![
                ("path", string_prop("Path to the .xlsx file", true)),
                (
                    "config",
                    string_prop(
                        "JSON chart config: {sheet, chart_type, categories_range, values_range, title?, row?, col?, ...}",
                        true,
                    ),
                ),
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
    handlers.insert("excel_chart_create".into(), handle_create);
}

fn handle_create(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let config_str = get_string(&args, "config").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    let config: ChartConfig = match serde_json::from_str(&config_str) {
        Ok(c) => c,
        Err(e) => return format!("Error parsing chart config JSON: {e}"),
    };

    let params = security_params(&path, dry_run);

    match excel_core::excel_write::add_chart(&path, &params, &config) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}
