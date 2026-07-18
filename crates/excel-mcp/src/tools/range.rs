// Range category tools: read, write, write_from_csv, clear.

use serde_json::Value;
use std::collections::HashMap;

use super::helpers::*;
use crate::server::{ToolDef, ToolHandler};
use excel_core::types::{CellValue, ReadRangeOptions, SecurityParams};

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_range_read",
            description: "Read a range of cells. Supports output modes: detailed (default), compact, csv.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("range", string_prop("Range like A1:C10", true)),
                    (
                        "mode",
                        string_prop(
                            "Output mode: detailed, compact, or csv (default: detailed)",
                            false,
                        ),
                    ),
                    ("truncate", int_prop("Max rows to return (optional)")),
                ],
                vec!["path", "sheet", "range"],
            ),
        },
        ToolDef {
            name: "excel_range_write",
            description: "Write data to a range of cells. Data should be a JSON array of arrays of values.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("range", string_prop("Starting cell like A1", true)),
                    ("data", string_prop("JSON string of 2D array data", true)),
                    (
                        "dry_run",
                        bool_prop("If true, simulate without writing", Some(false)),
                    ),
                ],
                vec!["path", "sheet", "range", "data"],
            ),
        },
        ToolDef {
            name: "excel_range_write_csv",
            description: "Write CSV file content to a range of cells.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("range", string_prop("Starting cell like A1", true)),
                    (
                        "csv_path",
                        string_prop("Path to the CSV file to import", true),
                    ),
                    (
                        "dry_run",
                        bool_prop("If true, simulate without writing", Some(false)),
                    ),
                ],
                vec!["path", "sheet", "range", "csv_path"],
            ),
        },
        ToolDef {
            name: "excel_range_clear",
            description: "Clear (empty) a range of cells.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("range", string_prop("Range like A1:C10", true)),
                    (
                        "dry_run",
                        bool_prop("If true, simulate without writing", Some(false)),
                    ),
                ],
                vec!["path", "sheet", "range"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_range_read".into(), handle_range_read);
    handlers.insert("excel_range_write".into(), handle_range_write);
    handlers.insert("excel_range_write_csv".into(), handle_range_write_csv);
    handlers.insert("excel_range_clear".into(), handle_range_clear);
}

fn params(path: &str, dry_run: bool) -> SecurityParams {
    security_params(path, dry_run)
}

fn handle_range_read(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let range = get_string(&args, "range").unwrap_or_default();
    let mode = get_string(&args, "mode").unwrap_or_else(|| "detailed".into());
    let truncate = get_u32(&args, "truncate").map(|n| n as usize);

    let output_mode = match mode.as_str() {
        "compact" => excel_core::types::OutputMode::Compact,
        "csv" => excel_core::types::OutputMode::Csv,
        _ => excel_core::types::OutputMode::Detailed,
    };

    let options = ReadRangeOptions {
        mode: output_mode,
        truncate,
        ..Default::default()
    };

    match excel_core::excel_read::read_range_with_options(&path, &sheet, &range, &options) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_range_write(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let range = get_string(&args, "range").unwrap_or_default();
    let data_str = get_string(&args, "data").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    let raw_data: Vec<Vec<String>> = match serde_json::from_str(&data_str) {
        Ok(d) => d,
        Err(e) => return format!("Error parsing data JSON: {e}"),
    };

    let data: Vec<Vec<CellValue>> = raw_data
        .iter()
        .map(|row| {
            row.iter()
                .map(|v| {
                    if v.is_empty() {
                        CellValue::Empty
                    } else if let Ok(n) = v.parse::<f64>() {
                        CellValue::Number(n)
                    } else if v.eq_ignore_ascii_case("true") {
                        CellValue::Bool(true)
                    } else if v.eq_ignore_ascii_case("false") {
                        CellValue::Bool(false)
                    } else {
                        CellValue::String(v.clone())
                    }
                })
                .collect()
        })
        .collect();

    match excel_core::excel_write::write_range(
        &path,
        &params(&path, dry_run),
        &sheet,
        &range,
        &data,
    ) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_range_write_csv(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let range = get_string(&args, "range").unwrap_or_default();
    let csv_path = get_string(&args, "csv_path").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    match excel_core::excel_write::write_range_from_csv(
        &path,
        &params(&path, dry_run),
        &sheet,
        &range,
        &csv_path,
    ) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_range_clear(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let range = get_string(&args, "range").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    match excel_core::excel_write::clear_range(&path, &params(&path, dry_run), &sheet, &range) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}
