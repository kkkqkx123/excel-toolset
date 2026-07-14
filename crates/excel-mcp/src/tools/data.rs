// Data category tools: append_row, insert_row, delete_row, filter, sort, dedup, sql.

use std::collections::HashMap;
use serde_json::Value;

use crate::server::{ToolDef, ToolHandler};
use excel_core::types::{CellValue, FilterCondition, FilterOp, SecurityParams, SortColumn};
use super::helpers::*;

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_data_append_row",
            description: "Append a row of data to the end of a sheet.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("values", string_array_prop("Array of values for the new row")),
                    ("dry_run", bool_prop("If true, simulate without writing", Some(false))),
                ],
                vec!["path", "sheet", "values"],
            ),
        },
        ToolDef {
            name: "excel_data_insert_row",
            description: "Insert a row at a specific position (1-indexed).",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("row", int_prop("Row number to insert at (1-indexed)")),
                    ("values", string_array_prop("Array of values for the new row")),
                    ("dry_run", bool_prop("If true, simulate without writing", Some(false))),
                ],
                vec!["path", "sheet", "row", "values"],
            ),
        },
        ToolDef {
            name: "excel_data_delete_row",
            description: "Delete a row at a specific position (1-indexed).",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("row", int_prop("Row number to delete (1-indexed)")),
                    ("dry_run", bool_prop("If true, simulate without writing", Some(false))),
                ],
                vec!["path", "sheet", "row"],
            ),
        },
        ToolDef {
            name: "excel_data_filter",
            description: "Filter rows by column condition. Operators: eq, neq, gt, gte, lt, lte, contains, startswith, endswith.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("column", int_prop("Column number (1-indexed)")),
                    ("op", enum_prop("Filter operator", &["eq","neq","gt","gte","lt","lte","contains","startswith","endswith"])),
                    ("value", string_prop("Value to compare against", true)),
                ],
                vec!["path", "sheet", "column", "op", "value"],
            ),
        },
        ToolDef {
            name: "excel_data_sort",
            description: "Sort rows by a column.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("column", int_prop("Column number to sort by (1-indexed)")),
                    ("desc", bool_prop("Sort descending (default: false)", Some(false))),
                    ("dry_run", bool_prop("If true, simulate without writing", Some(false))),
                ],
                vec!["path", "sheet", "column"],
            ),
        },
        ToolDef {
            name: "excel_data_dedup",
            description: "Remove duplicate rows. Optionally check only a specific column (0-indexed).",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name", true)),
                    ("column", int_prop("Optional: only check this column for duplicates (0-indexed)")),
                    ("dry_run", bool_prop("If true, simulate without writing", Some(false))),
                ],
                vec!["path", "sheet"],
            ),
        },
        ToolDef {
            name: "excel_data_sql",
            description: "Query Excel data using DuckDB SQL. Requires building with --features sql.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Sheet name to register as table", true)),
                    ("query", string_prop("SQL query string", true)),
                ],
                vec!["path", "sheet", "query"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_data_append_row".into(), handle_append_row);
    handlers.insert("excel_data_insert_row".into(), handle_insert_row);
    handlers.insert("excel_data_delete_row".into(), handle_delete_row);
    handlers.insert("excel_data_filter".into(), handle_filter);
    handlers.insert("excel_data_sort".into(), handle_sort);
    handlers.insert("excel_data_dedup".into(), handle_dedup);
    handlers.insert("excel_data_sql".into(), handle_sql);
}

fn params(path: &str, dry_run: bool) -> SecurityParams {
    security_params(path, dry_run)
}

fn strings_to_cell_values(values: &[String]) -> Vec<CellValue> {
    values.iter().map(|v| string_to_cell_value(v)).collect()
}

fn string_to_cell_value(s: &str) -> CellValue {
    if s.is_empty() { return CellValue::Empty; }
    if s.eq_ignore_ascii_case("true") { return CellValue::Bool(true); }
    if s.eq_ignore_ascii_case("false") { return CellValue::Bool(false); }
    if let Ok(n) = s.parse::<f64>() { return CellValue::Number(n); }
    CellValue::String(s.to_string())
}

fn parse_filter_op(op: &str) -> FilterOp {
    match op {
        "eq" => FilterOp::Eq, "neq" => FilterOp::Ne,
        "gt" => FilterOp::Gt, "gte" => FilterOp::Ge,
        "lt" => FilterOp::Lt, "lte" => FilterOp::Le,
        "contains" => FilterOp::Contains,
        "startswith" => FilterOp::StartsWith,
        "endswith" => FilterOp::EndsWith,
        _ => FilterOp::Eq,
    }
}

fn handle_append_row(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let values = get_string_array(&args, "values").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    let row: Vec<Vec<CellValue>> = vec![strings_to_cell_values(&values)];

    match excel_core::excel_write::append_rows(&path, &params(&path, dry_run), &sheet, &row) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_insert_row(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let row = get_u32(&args, "row").unwrap_or(1).saturating_sub(1);
    let values = get_string_array(&args, "values").unwrap_or_default();
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    let new_rows: Vec<Vec<CellValue>> = vec![strings_to_cell_values(&values)];

    match excel_core::excel_write::insert_rows(&path, &params(&path, dry_run), &sheet, row, &new_rows) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_delete_row(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let row = get_u32(&args, "row").unwrap_or(1).saturating_sub(1);
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    match excel_core::excel_write::delete_rows(&path, &params(&path, dry_run), &sheet, row, row) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_filter(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let column = get_u32(&args, "column").unwrap_or(1).saturating_sub(1) as u16;
    let op = get_string(&args, "op").unwrap_or_default();
    let value = get_string(&args, "value").unwrap_or_default();

    let conditions = vec![FilterCondition {
        column,
        operator: parse_filter_op(&op),
        value,
    }];

    match excel_core::operations::filter_rows(&path, &sheet, &conditions) {
        Ok(data) => to_result_string(&data),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_sort(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let column = get_u32(&args, "column").unwrap_or(1).saturating_sub(1) as u16;
    let desc = get_bool(&args, "desc").unwrap_or(false);
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    let sort_columns = vec![SortColumn { column, descending: desc }];

    match excel_core::operations::sort_sheet(&path, &params(&path, dry_run), &sheet, &sort_columns) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_dedup(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let column = get_u32(&args, "column");
    let dry_run = get_bool(&args, "dry_run").unwrap_or(false);

    let columns: Vec<u16> = column.map(|c| c as u16).into_iter().collect();

    match excel_core::operations::dedup_sheet(&path, &params(&path, dry_run), &sheet, &columns) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_sql(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let query = get_string(&args, "query").unwrap_or_default();

    match excel_core::operations::sql_query(&path, &sheet, &query) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}
