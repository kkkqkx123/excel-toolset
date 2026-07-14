// Conditional format category tools.

use std::collections::HashMap;
use serde_json::Value;

use crate::server::{ToolDef, ToolHandler};
use excel_core::features::conditional_format::{
    ConditionalFormatRule, parse_rule_type,
};
use super::helpers::*;

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_conditional_format_add",
            description: "Add conditional formatting to a range. Config is a JSON object with range, rule_type, condition, style?, config?.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("range", string_prop("Target range like A1:C10", true)),
                    ("rule_type", enum_prop("Rule type", &["cell_value","formula","duplicate","above_average","top10","text_contains","date_occurring","databar","colorscale","iconset"])),
                    ("condition", string_prop("Condition expression", true)),
                    ("style", string_prop("JSON style to apply", false)),
                    ("config", string_prop("JSON config for DataBar/ColorScale/IconSet types", false)),
                    ("dry_run", bool_prop("If true, simulate without writing", Some(false))),
                ],
                vec!["path", "sheet", "range", "rule_type", "condition"],
            ),
        },
        ToolDef {
            name: "excel_conditional_format_remove",
            description: "Remove conditional formatting from a range.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("range", string_prop("Range to remove formatting from", true)),
                    ("dry_run", bool_prop("If true, simulate without writing", Some(false))),
                ],
                vec!["path", "sheet", "range"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_conditional_format_add".into(), handle_add);
    handlers.insert("excel_conditional_format_remove".into(), handle_remove);
}


fn handle_add(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let range = get_string(&args, "range").unwrap_or_default();
    let rule_type = get_string(&args, "rule_type").unwrap_or_default();
    let condition = get_string(&args, "condition").unwrap_or_default();
    let style_str = get_string(&args, "style");
    let config_str = get_string(&args, "config");
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    let format = match style_str {
        Some(s) => match serde_json::from_str(&s) {
            Ok(fmt) => Some(fmt),
            Err(e) => return format!("Error parsing style JSON: {e}"),
        },
        None => None,
    };

    let config = match config_str {
        Some(s) => match serde_json::from_str(&s) {
            Ok(cfg) => Some(cfg),
            Err(e) => return format!("Error parsing config JSON: {e}"),
        },
        None => None,
    };

    let rule = ConditionalFormatRule {
        rule_type: parse_rule_type(&rule_type),
        condition,
        format,
        config,
    };

    match excel_core::features::conditional_format::add_conditional_format(
        &path, &sheet, &range, &rule, &security_params(&path, dry_run),
    ) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_remove(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let range = get_string(&args, "range").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    match excel_core::features::conditional_format::remove_conditional_format(&path, &sheet, &range, &security_params(&path, dry_run)) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}
