// Formula category tools.

use std::collections::HashMap;
use serde_json::Value;

use crate::server::{ToolDef, ToolHandler};
use super::helpers::*;

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_formula_set",
            description: "Set a formula in a cell.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("cell", string_prop("Cell reference like A1", true)),
                    ("formula", string_prop("Formula string like =SUM(A1:A10)", true)),
                    ("dry_run", bool_prop("If true, simulate without writing", Some(false))),
                ],
                vec!["path", "sheet", "cell", "formula"],
            ),
        },
        ToolDef {
            name: "excel_formula_refresh",
            description: "Refresh all formulas in a sheet.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name (use * for all sheets)", true)),
                    ("dry_run", bool_prop("If true, simulate without writing", Some(false))),
                ],
                vec!["path", "sheet"],
            ),
        },
        ToolDef {
            name: "excel_formula_read",
            description: "Read the formula (if any) from a cell.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("cell", string_prop("Cell reference like A1", true)),
                ],
                vec!["path", "sheet", "cell"],
            ),
        },
        ToolDef {
            name: "excel_formula_calc_mode",
            description: "Set workbook calculation mode (auto, manual). Note: currently writes unchanged data back.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("mode", enum_prop("Calculation mode", &["auto", "manual"])),
                    ("dry_run", bool_prop("If true, simulate without writing", Some(false))),
                ],
                vec!["path", "mode"],
            ),
        },
        ToolDef {
            name: "excel_formula_trace",
            description: "Trace formula dependencies: show which cells a formula references.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("cell", string_prop("Cell reference to trace", true)),
                ],
                vec!["path", "sheet", "cell"],
            ),
        },
        ToolDef {
            name: "excel_formula_explain",
            description: "Explain a formula in natural language.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("cell", string_prop("Cell reference to explain", true)),
                    ("language", enum_prop("Output language", &["en", "zh"])),
                ],
                vec!["path", "sheet", "cell"],
            ),
        },
        ToolDef {
            name: "excel_formula_explain_logic",
            description: "Explain the business logic behind a formula in natural language.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("cell", string_prop("Cell reference to explain", true)),
                    ("language", enum_prop("Output language", &["en", "zh"])),
                ],
                vec!["path", "sheet", "cell"],
            ),
        },
        ToolDef {
            name: "excel_formula_fill",
            description: "Auto-fill a formula from source cell to a target range.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("source", string_prop("Source cell with formula", true)),
                    ("target_range", string_prop("Target range to fill", true)),
                    ("dry_run", bool_prop("If true, simulate without writing", Some(false))),
                ],
                vec!["path", "sheet", "source", "target_range"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_formula_set".into(), handle_set);
    handlers.insert("excel_formula_refresh".into(), handle_refresh);
    handlers.insert("excel_formula_read".into(), handle_read);
    handlers.insert("excel_formula_calc_mode".into(), handle_calc_mode);
    handlers.insert("excel_formula_trace".into(), handle_trace);
    handlers.insert("excel_formula_explain".into(), handle_explain);
    handlers.insert("excel_formula_explain_logic".into(), handle_explain_logic);
    handlers.insert("excel_formula_fill".into(), handle_fill);
}


fn handle_set(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let cell = get_string(&args, "cell").unwrap_or_default();
    let formula = get_string(&args, "formula").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    match excel_core::excel_write::set_formula(&path, &security_params(&path, dry_run), &sheet, &cell, &formula) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_refresh(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    match excel_core::excel_write::refresh_formulas(&path, &security_params(&path, dry_run), &sheet) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_read(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let cell = get_string(&args, "cell").unwrap_or_default();
    let (row, col) = match parse_cell_ref(&cell) {
        Ok(rc) => rc,
        Err(e) => return format!("Error: {e}"),
    };

    match excel_core::excel_read::read_cell(&path, &sheet, row, col) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_calc_mode(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let mode = get_string(&args, "mode").unwrap_or_else(|| "auto".into());
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    match excel_core::excel_write::set_calculation_mode(&path, &security_params(&path, dry_run), &mode) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_trace(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let cell = get_string(&args, "cell").unwrap_or_default();

    match excel_core::features::formula_analysis::trace_dependencies(&path, &sheet, &cell) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_explain(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let cell = get_string(&args, "cell").unwrap_or_default();
    let language = get_string(&args, "language").unwrap_or_else(|| "en".into());

    match excel_core::features::formula_analysis::explain_formula(&path, &sheet, &cell, &language) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_explain_logic(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let cell = get_string(&args, "cell").unwrap_or_default();
    let language = get_string(&args, "language").unwrap_or_else(|| "en".into());

    match excel_core::features::formula_analysis::explain_formula_logic(&path, &sheet, &cell, &language) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_fill(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let source = get_string(&args, "source").unwrap_or_default();
    let target_range = get_string(&args, "target_range").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    match excel_core::features::formula_ops::fill_formula(&path, &sheet, &source, &target_range, &security_params(&path, dry_run)) {
        Ok(r) => to_result_string(&r),
        Err(e) => format!("Error: {e}"),
    }
}

fn parse_cell_ref(cell: &str) -> Result<(u32, u16), String> {
    let col_letters: String = cell.chars().take_while(|c| c.is_alphabetic()).collect();
    let row_str: String = cell.chars().skip_while(|c| c.is_alphabetic()).collect();
    if col_letters.is_empty() || row_str.is_empty() {
        return Err(format!("Invalid cell reference: {cell}"));
    }
    let col = col_letters.to_uppercase().chars()
        .fold(0u16, |acc, c| acc * 26 + (c as u16 - b'A' as u16 + 1))
        .saturating_sub(1);
    let row = row_str.parse::<u32>().map_err(|_| format!("Invalid row number in: {cell}"))?;
    Ok((row.saturating_sub(1), col))
}
