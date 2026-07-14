// Diff category tools: file, range, semantic, formula_deps.

use std::collections::HashMap;
use serde_json::Value;

use crate::server::{ToolDef, ToolHandler};
use super::helpers::*;

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_diff_file",
            description: "Compare two Excel files and return all cell-level differences.",
            input_schema: object_schema(
                vec![
                    ("old_path", string_prop("Path to the old .xlsx file", true)),
                    ("new_path", string_prop("Path to the new .xlsx file", true)),
                    ("sheet", string_prop("Optional: only diff a specific sheet", false)),
                ],
                vec!["old_path", "new_path"],
            ),
        },
        ToolDef {
            name: "excel_diff_range",
            description: "Compare a specific range between two Excel files.",
            input_schema: object_schema(
                vec![
                    ("old_path", string_prop("Path to the old .xlsx file", true)),
                    ("new_path", string_prop("Path to the new .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("range", string_prop("Range to compare like A1:C10", true)),
                ],
                vec!["old_path", "new_path", "sheet", "range"],
            ),
        },
        ToolDef {
            name: "excel_diff_semantic",
            description: "Generate a semantic (natural language) diff summary between two Excel files.",
            input_schema: object_schema(
                vec![
                    ("old_path", string_prop("Path to the old .xlsx file", true)),
                    ("new_path", string_prop("Path to the new .xlsx file", true)),
                ],
                vec!["old_path", "new_path"],
            ),
        },
        ToolDef {
            name: "excel_diff_formula_deps",
            description: "Compare formula dependency graphs between two Excel files.",
            input_schema: object_schema(
                vec![
                    ("old_path", string_prop("Path to the old .xlsx file", true)),
                    ("new_path", string_prop("Path to the new .xlsx file", true)),
                    ("sheet", string_prop("Sheet name to analyze", true)),
                ],
                vec!["old_path", "new_path", "sheet"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_diff_file".into(), handle_file);
    handlers.insert("excel_diff_range".into(), handle_range);
    handlers.insert("excel_diff_semantic".into(), handle_semantic);
    handlers.insert("excel_diff_formula_deps".into(), handle_formula_deps);
}

fn handle_file(args: Value) -> String {
    let old_path = get_string(&args, "old_path").unwrap_or_default();
    let new_path = get_string(&args, "new_path").unwrap_or_default();
    let sheet = get_string(&args, "sheet");

    match sheet {
        Some(s) => match excel_diff::diff_sheets(&old_path, &new_path, &s) {
            Ok(r) => to_result_string(&r),
            Err(e) => format!("Error: {e}"),
        },
        None => match excel_diff::diff_files(&old_path, &new_path) {
            Ok(r) => to_result_string(&r),
            Err(e) => format!("Error: {e}"),
        },
    }
}

fn handle_range(args: Value) -> String {
    let old_path = get_string(&args, "old_path").unwrap_or_default();
    let new_path = get_string(&args, "new_path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let range = get_string(&args, "range").unwrap_or_default();

    match excel_diff::diff_range(&old_path, &new_path, &sheet, &range) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_semantic(args: Value) -> String {
    let old_path = get_string(&args, "old_path").unwrap_or_default();
    let new_path = get_string(&args, "new_path").unwrap_or_default();

    match excel_diff::diff_with_semantic(&old_path, &new_path) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_formula_deps(args: Value) -> String {
    let old_path = get_string(&args, "old_path").unwrap_or_default();
    let new_path = get_string(&args, "new_path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();

    match excel_diff::diff_formula_dependencies(&old_path, &new_path, &sheet) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}
