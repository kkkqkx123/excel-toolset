// Cell category tools: read, write.

use std::collections::HashMap;
use serde_json::Value;

use crate::server::{ToolDef, ToolHandler};
use excel_core::types::CellValue;
use super::helpers::*;

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_cell_read",
            description: "Read the value of a single cell. Cell reference like 'A1' or 'B2'.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("cell", string_prop("Cell reference, e.g. A1", true)),
                ],
                vec!["path", "sheet", "cell"],
            ),
        },
        ToolDef {
            name: "excel_cell_write",
            description: "Write a value to a single cell.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("cell", string_prop("Cell reference, e.g. A1", true)),
                    ("value", string_prop("Value to write", true)),
                    ("dry_run", bool_prop("If true, simulate without writing", Some(false))),
                ],
                vec!["path", "sheet", "cell", "value"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_cell_read".into(), handle_cell_read);
    handlers.insert("excel_cell_write".into(), handle_cell_write);
}

fn handle_cell_read(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let cell = get_string(&args, "cell").unwrap_or_default();

    let (row, col) = match parse_cell(&cell) {
        Ok(rc) => rc,
        Err(e) => return format!("Error: {e}"),
    };

    match excel_core::excel_read::read_cell(&path, &sheet, row, col) {
        Ok(data) => to_result_string(&data),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_cell_write(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let cell = get_string(&args, "cell").unwrap_or_default();
    let value = get_string(&args, "value").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    let (row, col) = match parse_cell(&cell) {
        Ok(rc) => rc,
        Err(e) => return format!("Error: {e}"),
    };

    let params = security_params(&path, dry_run);

    let cell_value = string_to_cell_value(&value);

    match excel_core::excel_write::write_cell(&path, &params, &sheet, row, col, &cell_value) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn parse_cell(cell: &str) -> Result<(u32, u16), String> {
    let col_letters: String = cell.chars().take_while(|c| c.is_alphabetic()).collect();
    let row_str: String = cell.chars().skip_while(|c| c.is_alphabetic()).collect();

    if col_letters.is_empty() || row_str.is_empty() {
        return Err(format!("Invalid cell reference: {cell}"));
    }

    let col = col_letters
        .to_uppercase()
        .chars()
        .fold(0u16, |acc, c| acc * 26 + (c as u16 - b'A' as u16 + 1))
        .saturating_sub(1);

    let row = row_str.parse::<u32>().map_err(|_| format!("Invalid row number in: {cell}"))?;

    Ok((row.saturating_sub(1), col))
}

/// Convert a string to CellValue, auto-detecting numbers and booleans.
fn string_to_cell_value(s: &str) -> CellValue {
    if s.is_empty() {
        return CellValue::Empty;
    }
    if s.eq_ignore_ascii_case("true") {
        return CellValue::Bool(true);
    }
    if s.eq_ignore_ascii_case("false") {
        return CellValue::Bool(false);
    }
    if let Ok(n) = s.parse::<f64>() {
        return CellValue::Number(n);
    }
    CellValue::String(s.to_string())
}
