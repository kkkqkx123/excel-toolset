//! Dynamic array spill handling.
//!
//! When a formula returns an array of multiple values, Excel "spills" them
//! into adjacent cells. This module handles spill computation and conflict detection.

use std::collections::HashSet;

use excel_types::CellValue;

use crate::engine::DataProvider;
use crate::evaluator::{EvalError, EvalResult, Evaluator, partial_cmp_cell_values, to_number};
use crate::types::{AstNode, BinOp};

/// Result of a dynamic array spill.
#[derive(Debug, Clone)]
pub struct SpillResult {
    /// The 2D array of spilled values (rows x cols).
    pub values: Vec<Vec<CellValue>>,
    /// Number of rows in the spill region.
    pub rows: usize,
    /// Number of columns in the spill region.
    pub cols: usize,
}

impl SpillResult {
    pub fn single(value: CellValue) -> Self {
        Self {
            values: vec![vec![value]],
            rows: 1,
            cols: 1,
        }
    }

    pub fn from_2d(values: Vec<Vec<CellValue>>) -> Self {
        let rows = values.len();
        let cols = values.first().map_or(0, |r| r.len());
        Self { values, rows, cols }
    }
}

/// Check if a function name indicates it's a spill-capable dynamic array function.
pub fn is_spill_function(name: &str) -> bool {
    matches!(
        name,
        "FILTER" | "SORT" | "SORTBY" | "UNIQUE" | "SEQUENCE" | "RANDARRAY"
    )
}

/// Attempt to compute the spill result for a dynamic array function.
///
/// For functions like FILTER, SORT, UNIQUE, etc., this computes the full
/// spilled array and returns it as a SpillResult.
pub fn try_spill<P: DataProvider>(
    sheet: &str,
    node: &AstNode,
    evaluator: &mut Evaluator<'_, P>,
) -> EvalResult<SpillResult> {
    match node {
        AstNode::Function { name, args } => match name.as_str() {
            "FILTER" => try_spill_filter(sheet, args, evaluator),
            "SORT" => try_spill_sort(sheet, args, evaluator),
            "SORTBY" => try_spill_sortby(sheet, args, evaluator),
            "UNIQUE" => try_spill_unique(sheet, args, evaluator),
            "SEQUENCE" => try_spill_sequence(sheet, args, evaluator),
            "RANDARRAY" => try_spill_randarray(sheet, args, evaluator),
            _ => {
                let val = evaluator.evaluate(sheet, node)?;
                Ok(SpillResult::single(val))
            }
        },
        _ => {
            let val = evaluator.evaluate(sheet, node)?;
            Ok(SpillResult::single(val))
        }
    }
}

/// Extract a 2D array from an AST node (Range or evaluated value).
fn ast_to_2d<P: DataProvider>(
    sheet: &str,
    node: &AstNode,
    evaluator: &mut Evaluator<'_, P>,
) -> EvalResult<Vec<Vec<CellValue>>> {
    match node {
        AstNode::Range(range) => {
            let sheet_name = range.start.sheet.as_deref().unwrap_or(sheet);
            Ok(evaluator.eval_range_to_2d(
                sheet_name,
                range.start.row,
                range.start.col,
                range.end.row,
                range.end.col,
            ))
        }
        AstNode::Array(rows) => {
            let mut result: Vec<Vec<CellValue>> = Vec::new();
            for row in rows {
                let mut row_vals = Vec::new();
                for cell in row {
                    row_vals.push(evaluator.evaluate(sheet, cell)?);
                }
                result.push(row_vals);
            }
            Ok(result)
        }
        _ => {
            let val = evaluator.evaluate(sheet, node)?;
            Ok(vec![vec![val]])
        }
    }
}

fn compare_cell_values(a: &CellValue, b: &CellValue) -> std::cmp::Ordering {
    match partial_cmp_cell_values(a, b) {
        Some(ord) => ord,
        None => std::cmp::Ordering::Equal,
    }
}

fn try_spill_filter<P: DataProvider>(
    sheet: &str,
    args: &[AstNode],
    evaluator: &mut Evaluator<'_, P>,
) -> EvalResult<SpillResult> {
    if args.is_empty() {
        return Err(EvalError::new("FILTER requires at least 1 argument"));
    }

    let array_node = &args[0];

    match array_node {
        AstNode::Range(range) => {
            let sheet_name = range.start.sheet.as_deref().unwrap_or(sheet);
            let data = evaluator.eval_range_to_2d(
                sheet_name,
                range.start.row,
                range.start.col,
                range.end.row,
                range.end.col,
            );

            let include = if args.len() > 1 {
                evaluate_filter_condition(&args[1], &data, sheet, evaluator)?
            } else {
                vec![true; data.len()]
            };

            let if_empty = if args.len() > 2 {
                match &args[2] {
                    AstNode::String(s) => s.clone(),
                    _ => "".to_string(),
                }
            } else {
                "No results".to_string()
            };

            let mut result: Vec<Vec<CellValue>> = Vec::new();
            let headers = if !data.is_empty() {
                &data[0]
            } else {
                return Ok(SpillResult::from_2d(result));
            };

            result.push(headers.clone());

            for (i, row) in data.iter().skip(1).enumerate() {
                if i < include.len() && include[i] {
                    result.push(row.clone());
                }
            }

            if result.len() == 1 {
                let empty_msg = vec![vec![CellValue::String(if_empty)]];
                return Ok(SpillResult::from_2d(empty_msg));
            }

            Ok(SpillResult::from_2d(result))
        }
        _ => {
            let val = evaluator.evaluate(sheet, array_node)?;
            Ok(SpillResult::single(val))
        }
    }
}

/// SORT(array, [sort_index], [sort_order], [by_col])
fn try_spill_sort<P: DataProvider>(
    sheet: &str,
    args: &[AstNode],
    evaluator: &mut Evaluator<'_, P>,
) -> EvalResult<SpillResult> {
    if args.is_empty() {
        return Err(EvalError::new("SORT requires at least 1 argument"));
    }

    let data = ast_to_2d(sheet, &args[0], evaluator)?;
    if data.is_empty() {
        return Ok(SpillResult::from_2d(data));
    }

    // sort_index (1-based), default 1
    let sort_index = if args.len() > 1 {
        evaluate_to_f64(sheet, &args[1], evaluator).unwrap_or(1.0) as usize
    } else {
        1
    };
    // sort_order: 1 = ascending (default), -1 = descending
    let sort_order = if args.len() > 2 {
        evaluate_to_f64(sheet, &args[2], evaluator).unwrap_or(1.0)
    } else {
        1.0
    };
    // by_col: false = sort by row (default), true = sort by column
    let by_col = if args.len() > 3 {
        evaluate_to_bool(sheet, &args[3], evaluator)
    } else {
        false
    };

    let ascending = sort_order >= 0.0;

    let mut result = data.clone();

    if by_col {
        // Sort columns based on a row
        if result.is_empty() || result[0].is_empty() {
            return Ok(SpillResult::from_2d(result));
        }
        let row_idx = (sort_index.saturating_sub(1)).min(result.len().saturating_sub(1));
        let n_cols = result[0].len();

        // Build column-major data, sort, then transpose back
        let mut cols: Vec<Vec<CellValue>> = Vec::new();
        for c in 0..n_cols {
            let mut col_data = Vec::new();
            for r in 0..result.len() {
                col_data.push(result[r][c].clone());
            }
            cols.push(col_data);
        }

        let mut indices: Vec<usize> = (0..n_cols).collect();
        let ref_row = &result[row_idx];
        indices.sort_by(|&a, &b| {
            let ord = compare_cell_values(&ref_row[a], &ref_row[b]);
            if ascending { ord } else { ord.reverse() }
        });

        let mut sorted: Vec<Vec<CellValue>> = Vec::new();
        for r in 0..result.len() {
            let mut new_row = Vec::new();
            for &idx in &indices {
                new_row.push(result[r][idx].clone());
            }
            sorted.push(new_row);
        }
        result = sorted;
    } else {
        // Sort rows by a column
        let col_idx = (sort_index.saturating_sub(1))
            .min(result.first().map_or(0, |r| r.len()).saturating_sub(1));

        result.sort_by(|a, b| {
            let va = a.get(col_idx).unwrap_or(&CellValue::Empty);
            let vb = b.get(col_idx).unwrap_or(&CellValue::Empty);
            let ord = compare_cell_values(va, vb);
            if ascending { ord } else { ord.reverse() }
        });
    }

    Ok(SpillResult::from_2d(result))
}

/// SORTBY(array, by_array1, [sort_order1], [by_array2], [sort_order2], ...)
fn try_spill_sortby<P: DataProvider>(
    sheet: &str,
    args: &[AstNode],
    evaluator: &mut Evaluator<'_, P>,
) -> EvalResult<SpillResult> {
    if args.len() < 2 {
        return Err(EvalError::new("SORTBY requires at least 2 arguments"));
    }

    let data = ast_to_2d(sheet, &args[0], evaluator)?;
    if data.is_empty() {
        return Ok(SpillResult::from_2d(data));
    }

    let n_rows = data.len();

    // Parse by_arrays and sort_orders
    // args are interleaved: by_array1, [sort_order1], by_array2, [sort_order2], ...
    let mut by_arrays: Vec<Vec<CellValue>> = Vec::new();
    let mut sort_orders: Vec<f64> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        let by_data = ast_to_2d(sheet, &args[i], evaluator)?;
        // Flatten to a single column
        let flat: Vec<CellValue> = by_data.into_iter().flatten().collect();
        by_arrays.push(flat);

        // Next arg is optional sort_order
        if i + 1 < args.len() {
            // Peek: if the next arg looks like a number, it's a sort_order
            if let Some(order) = evaluate_to_f64(sheet, &args[i + 1], evaluator) {
                sort_orders.push(order);
                i += 1;
            } else {
                sort_orders.push(1.0); // default ascending
            }
        } else {
            sort_orders.push(1.0);
        }
        i += 1;
    }

    // Create indices and sort
    let mut indices: Vec<usize> = (0..n_rows).collect();
    indices.sort_by(|&a, &b| {
        for (j, by_arr) in by_arrays.iter().enumerate() {
            let va = by_arr.get(a).unwrap_or(&CellValue::Empty);
            let vb = by_arr.get(b).unwrap_or(&CellValue::Empty);
            let ord = compare_cell_values(va, vb);
            if ord != std::cmp::Ordering::Equal {
                let ascending = sort_orders.get(j).copied().unwrap_or(1.0) >= 0.0;
                return if ascending { ord } else { ord.reverse() };
            }
        }
        std::cmp::Ordering::Equal
    });

    let mut result: Vec<Vec<CellValue>> = Vec::new();
    for idx in indices {
        if idx < data.len() {
            result.push(data[idx].clone());
        }
    }

    Ok(SpillResult::from_2d(result))
}

/// UNIQUE(array, [by_col], [exactly_once])
fn try_spill_unique<P: DataProvider>(
    sheet: &str,
    args: &[AstNode],
    evaluator: &mut Evaluator<'_, P>,
) -> EvalResult<SpillResult> {
    if args.is_empty() {
        return Err(EvalError::new("UNIQUE requires at least 1 argument"));
    }

    let data = ast_to_2d(sheet, &args[0], evaluator)?;
    if data.is_empty() {
        return Ok(SpillResult::from_2d(data));
    }

    let by_col = if args.len() > 1 {
        evaluate_to_bool(sheet, &args[1], evaluator)
    } else {
        false
    };
    let exactly_once = if args.len() > 2 {
        evaluate_to_bool(sheet, &args[2], evaluator)
    } else {
        false
    };

    if by_col {
        // Unique columns (transpose, dedup, transpose back)
        if data[0].is_empty() {
            return Ok(SpillResult::from_2d(data));
        }
        let n_rows = data.len();
        let n_cols = data[0].len();

        let mut cols: Vec<Vec<CellValue>> = Vec::new();
        for c in 0..n_cols {
            let mut col_data = Vec::new();
            for r in 0..n_rows {
                col_data.push(data[r][c].clone());
            }
            cols.push(col_data);
        }

        if exactly_once {
            // Count occurrences and keep only unique
            let mut col_keys: Vec<String> = Vec::new();
            for col in &cols {
                col_keys.push(row_key(col));
            }
            let mut counts: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for key in &col_keys {
                *counts.entry(key.clone()).or_insert(0) += 1;
            }
            let unique_cols: Vec<Vec<CellValue>> = cols
                .into_iter()
                .enumerate()
                .filter(|(i, _)| counts.get(&col_keys[*i]).copied() == Some(1))
                .map(|(_, c)| c)
                .collect();

            // Transpose back
            let mut result: Vec<Vec<CellValue>> = Vec::new();
            let out_cols = unique_cols.len();
            if out_cols == 0 {
                return Ok(SpillResult::from_2d(Vec::new()));
            }
            for r in 0..n_rows {
                let mut row_vals = Vec::new();
                for c in 0..out_cols {
                    row_vals.push(unique_cols[c][r].clone());
                }
                result.push(row_vals);
            }
            Ok(SpillResult::from_2d(result))
        } else {
            let mut seen: HashSet<String> = HashSet::new();
            let mut unique_cols: Vec<Vec<CellValue>> = Vec::new();
            for col in cols {
                let key = row_key(&col);
                if seen.insert(key) {
                    unique_cols.push(col);
                }
            }
            let out_cols = unique_cols.len();
            let mut result: Vec<Vec<CellValue>> = Vec::new();
            for r in 0..n_rows {
                let mut row_vals = Vec::new();
                for c in 0..out_cols {
                    row_vals.push(unique_cols[c][r].clone());
                }
                result.push(row_vals);
            }
            Ok(SpillResult::from_2d(result))
        }
    } else {
        // Unique rows
        if exactly_once {
            let mut row_keys_vec: Vec<String> = Vec::new();
            for row in &data {
                row_keys_vec.push(row_key(row));
            }
            let mut counts: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for key in &row_keys_vec {
                *counts.entry(key.clone()).or_insert(0) += 1;
            }
            let result: Vec<Vec<CellValue>> = data
                .into_iter()
                .enumerate()
                .filter(|(i, _)| counts.get(&row_keys_vec[*i]).copied() == Some(1))
                .map(|(_, r)| r)
                .collect();
            Ok(SpillResult::from_2d(result))
        } else {
            let mut seen: HashSet<String> = HashSet::new();
            let mut result: Vec<Vec<CellValue>> = Vec::new();
            for row in data {
                let key = row_key(&row);
                if seen.insert(key) {
                    result.push(row);
                }
            }
            Ok(SpillResult::from_2d(result))
        }
    }
}

/// SEQUENCE(rows, [columns], [start], [step])
fn try_spill_sequence<P: DataProvider>(
    sheet: &str,
    args: &[AstNode],
    evaluator: &mut Evaluator<'_, P>,
) -> EvalResult<SpillResult> {
    let rows = if !args.is_empty() {
        evaluate_to_f64(sheet, &args[0], evaluator).unwrap_or(1.0) as usize
    } else {
        1
    };
    let cols = if args.len() > 1 {
        evaluate_to_f64(sheet, &args[1], evaluator).unwrap_or(1.0) as usize
    } else {
        1
    };
    let start = if args.len() > 2 {
        evaluate_to_f64(sheet, &args[2], evaluator).unwrap_or(1.0)
    } else {
        1.0
    };
    let step = if args.len() > 3 {
        evaluate_to_f64(sheet, &args[3], evaluator).unwrap_or(1.0)
    } else {
        1.0
    };

    if rows == 0 || cols == 0 {
        return Ok(SpillResult::single(CellValue::Error("#VALUE!".into())));
    }

    let mut result: Vec<Vec<CellValue>> = Vec::with_capacity(rows);
    let mut current = start;

    for _ in 0..rows {
        let mut row_vals = Vec::with_capacity(cols);
        for _ in 0..cols {
            row_vals.push(CellValue::Number(current));
            current += step;
        }
        result.push(row_vals);
    }

    Ok(SpillResult::from_2d(result))
}

/// RANDARRAY([rows], [cols], [min], [max], [integer])
fn try_spill_randarray<P: DataProvider>(
    sheet: &str,
    args: &[AstNode],
    evaluator: &mut Evaluator<'_, P>,
) -> EvalResult<SpillResult> {
    let rows = if !args.is_empty() {
        evaluate_to_f64(sheet, &args[0], evaluator).unwrap_or(1.0) as usize
    } else {
        1
    };
    let cols = if args.len() > 1 {
        evaluate_to_f64(sheet, &args[1], evaluator).unwrap_or(1.0) as usize
    } else {
        1
    };
    let min_val = if args.len() > 2 {
        evaluate_to_f64(sheet, &args[2], evaluator).unwrap_or(0.0)
    } else {
        0.0
    };
    let max_val = if args.len() > 3 {
        evaluate_to_f64(sheet, &args[3], evaluator).unwrap_or(1.0)
    } else {
        1.0
    };
    let integer = if args.len() > 4 {
        evaluate_to_bool(sheet, &args[4], evaluator)
    } else {
        false
    };

    if rows == 0 || cols == 0 {
        return Ok(SpillResult::single(CellValue::Error("#VALUE!".into())));
    }

    let mut result: Vec<Vec<CellValue>> = Vec::with_capacity(rows);
    for _ in 0..rows {
        let mut row_vals = Vec::with_capacity(cols);
        for _ in 0..cols {
            let r: f64 = rand::random();
            let val = min_val + r * (max_val - min_val);
            if integer {
                row_vals.push(CellValue::Number((val).round()));
            } else {
                row_vals.push(CellValue::Number(val));
            }
        }
        result.push(row_vals);
    }

    Ok(SpillResult::from_2d(result))
}

/// Helper: evaluate an AST node to f64
fn evaluate_to_f64<P: DataProvider>(
    sheet: &str,
    node: &AstNode,
    evaluator: &mut Evaluator<'_, P>,
) -> Option<f64> {
    evaluator
        .evaluate(sheet, node)
        .ok()
        .and_then(|v| to_number(&v))
}

/// Helper: evaluate an AST node to bool
fn evaluate_to_bool<P: DataProvider>(
    sheet: &str,
    node: &AstNode,
    evaluator: &mut Evaluator<'_, P>,
) -> bool {
    match evaluator.evaluate(sheet, node) {
        Ok(CellValue::Bool(b)) => b,
        Ok(CellValue::Number(n)) => n != 0.0,
        Ok(CellValue::String(s)) => !s.is_empty() && s.to_uppercase() != "FALSE",
        _ => false,
    }
}

/// Create a string key for a row of CellValues (for dedup).
fn row_key(row: &[CellValue]) -> String {
    let parts: Vec<String> = row.iter().map(|v| format!("{:?}", v)).collect();
    parts.join("\x00")
}

fn evaluate_filter_condition<P: DataProvider>(
    condition: &AstNode,
    data: &[Vec<CellValue>],
    _sheet: &str,
    _evaluator: &mut Evaluator<'_, P>,
) -> EvalResult<Vec<bool>> {
    match condition {
        AstNode::BinaryOp {
            op: crate::types::BinOp::Gt,
            left: _,
            right,
        } => {
            if let AstNode::Number(threshold) = **right {
                let mut result = Vec::new();
                for row in data.iter().skip(1) {
                    if let Some(first_val) = row.first() {
                        if let Some(n) = to_number(first_val) {
                            result.push(n > threshold);
                        } else {
                            result.push(false);
                        }
                    } else {
                        result.push(false);
                    }
                }
                return Ok(result);
            }
        }
        _ => {}
    }
    Ok(vec![true; data.len().saturating_sub(1)])
}

// Extension trait for Evaluator to support range-to-2d conversion
impl<'a, P: DataProvider> Evaluator<'a, P> {
    pub fn eval_range_to_2d(
        &self,
        sheet: &str,
        start_row: u32,
        start_col: u32,
        end_row: u32,
        end_col: u32,
    ) -> Vec<Vec<CellValue>> {
        let mut result = Vec::new();
        for r in start_row..=end_row {
            let mut row_data = Vec::new();
            for c in start_col..=end_col {
                let val = self
                    .data_provider()
                    .get_cell(sheet, r, c)
                    .unwrap_or(CellValue::Empty);
                row_data.push(val);
            }
            result.push(row_data);
        }
        result
    }
}
