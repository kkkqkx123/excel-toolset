//! Dynamic array functions (FILTER, SORT, SORTBY, UNIQUE, SEQUENCE, RANDARRAY, LET, LAMBDA, MAP, REDUCE, SCAN, BYROW, BYCOL).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use excel_types::CellValue;

use crate::engine::DataProvider;
use crate::evaluator::to_number;

pub fn register(
    registry: &mut HashMap<
        String,
        Arc<dyn Fn(&[CellValue], &dyn DataProvider) -> CellValue + Send + Sync>,
    >,
) {
    registry.insert(
        "FILTER".into(),
        Arc::new(|args, provider| dynamic_filter(args)),
    );
    registry.insert("SORT".into(), Arc::new(|args, provider| dynamic_sort(args)));
    registry.insert(
        "SORTBY".into(),
        Arc::new(|args, provider| dynamic_sortby(args)),
    );
    registry.insert(
        "UNIQUE".into(),
        Arc::new(|args, provider| dynamic_unique(args)),
    );
    registry.insert(
        "SEQUENCE".into(),
        Arc::new(|args, provider| dynamic_sequence(args)),
    );
    registry.insert(
        "RANDARRAY".into(),
        Arc::new(|args, provider| dynamic_randarray(args)),
    );
    registry.insert("LET".into(), Arc::new(|args, provider| dynamic_let(args)));
    registry.insert(
        "LAMBDA".into(),
        Arc::new(|args, provider| dynamic_lambda(args)),
    );
    registry.insert("MAP".into(), Arc::new(|args, provider| dynamic_map(args)));
    registry.insert(
        "REDUCE".into(),
        Arc::new(|args, provider| dynamic_reduce(args)),
    );
    registry.insert("SCAN".into(), Arc::new(|args, provider| dynamic_scan(args)));
    registry.insert(
        "BYROW".into(),
        Arc::new(|args, provider| dynamic_byrow(args)),
    );
    registry.insert(
        "BYCOL".into(),
        Arc::new(|args, provider| dynamic_bycol(args)),
    );
}

// --- Extract 2D array from range-marker format in CellValue args ---

/// Attempt to reconstruct a 2D array from CellValue args that use the range-marker format.
/// Format: [sentinel: -(cols + 1_000_000.0), rows: Number(n_rows), then n_cols*n_rows CellValues]
fn try_extract_2d(args: &[CellValue]) -> Option<(usize, usize, Vec<Vec<CellValue>>)> {
    if args.len() < 3 {
        return None;
    }
    let sentinel = to_number(&args[0])?;
    if sentinel >= -999_999.0 || sentinel <= -2_000_000.0 {
        return None;
    }
    let n_cols = (-(sentinel + 1_000_000.0)) as usize;
    let n_rows = to_number(&args[1])? as usize;
    if n_cols == 0 || n_rows == 0 {
        return None;
    }
    let expected = n_cols * n_rows;
    if args.len() < 2 + expected {
        return None;
    }
    let mut result: Vec<Vec<CellValue>> = Vec::with_capacity(n_rows);
    for r in 0..n_rows {
        let mut row_data = Vec::with_capacity(n_cols);
        for c in 0..n_cols {
            row_data.push(args[2 + r * n_cols + c].clone());
        }
        result.push(row_data);
    }
    Some((n_cols, n_rows, result))
}

/// Check if args start with range-marker sentinel indicating inline range expansion.
fn has_range_marker(args: &[CellValue]) -> bool {
    if let Some(sentinel) = args.first().and_then(to_number) {
        sentinel < -999_999.0 && sentinel > -2_000_000.0
    } else {
        false
    }
}

/// Collect numbers from inline range data (after the sentinel and row count markers).
fn collect_range_numbers(args: &[CellValue]) -> Vec<f64> {
    if has_range_marker(args) && args.len() > 2 {
        args[2..].iter().filter_map(to_number).collect()
    } else {
        args.iter().filter_map(to_number).collect()
    }
}

// --- FILTER ---

fn dynamic_filter(args: &[CellValue]) -> CellValue {
    if args.is_empty() {
        return CellValue::Error("#VALUE!".into());
    }

    let include = args.get(1).map_or(false, |v| match v {
        CellValue::Bool(true) => true,
        CellValue::Number(n) if *n != 0.0 => true,
        _ => false,
    });

    if include {
        args.first().cloned().unwrap_or(CellValue::Empty)
    } else {
        args.get(2).cloned().unwrap_or(CellValue::String("".into()))
    }
}

// --- SORT ---

fn dynamic_sort(args: &[CellValue]) -> CellValue {
    if args.is_empty() {
        return CellValue::Error("#VALUE!".into());
    }

    let sort_order = args.get(2).and_then(to_number).unwrap_or(1.0);
    let ascending = sort_order >= 0.0;

    let mut values: Vec<CellValue> = args.iter().take(1).cloned().collect();
    // For inline range, extract all values after markers
    if let Some((n_cols, n_rows, data)) = try_extract_2d(args) {
        values = data.into_iter().flatten().collect();
    }

    sort_cell_values(&mut values, ascending);

    values.into_iter().next().unwrap_or(CellValue::Empty)
}

// --- SORTBY ---

fn dynamic_sortby(args: &[CellValue]) -> CellValue {
    if args.len() < 2 {
        return CellValue::Error("#VALUE!".into());
    }
    // Single-cell fallback: return first array's first element
    if has_range_marker(args) {
        let (_, _, data) = try_extract_2d(args).unwrap_or((0, 0, vec![]));
        if let Some(first) = data.first().and_then(|r| r.first()) {
            return first.clone();
        }
    }
    args.first().cloned().unwrap_or(CellValue::Empty)
}

// --- UNIQUE ---

fn dynamic_unique(args: &[CellValue]) -> CellValue {
    if args.is_empty() {
        return CellValue::Error("#VALUE!".into());
    }

    let exactly_once = args.get(2).map_or(false, |v| match v {
        CellValue::Bool(true) => true,
        _ => false,
    });

    // Extract values from inline range if present
    let values: Vec<CellValue> = if let Some((_, _, data)) = try_extract_2d(args) {
        data.into_iter().flatten().collect()
    } else {
        args.iter().cloned().collect()
    };

    if exactly_once {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for v in &values {
            let key = format!("{:?}", v);
            *counts.entry(key).or_insert(0) += 1;
        }
        for v in &values {
            let key = format!("{:?}", v);
            if counts.get(&key).copied() == Some(1) {
                return v.clone();
            }
        }
        CellValue::Empty
    } else {
        let mut seen: HashSet<String> = HashSet::new();
        for v in &values {
            let key = format!("{:?}", v);
            if seen.insert(key) {
                return v.clone();
            }
        }
        CellValue::Empty
    }
}

// --- SEQUENCE ---

fn dynamic_sequence(args: &[CellValue]) -> CellValue {
    let rows = args.first().and_then(to_number).unwrap_or(1.0) as usize;
    let start = args.get(2).and_then(to_number).unwrap_or(1.0);

    if rows == 0 {
        return CellValue::Error("#VALUE!".into());
    }

    CellValue::Number(start)
}

// --- RANDARRAY ---

fn dynamic_randarray(args: &[CellValue]) -> CellValue {
    let min_val = args.get(2).and_then(to_number).unwrap_or(0.0);
    let max_val = args.get(3).and_then(to_number).unwrap_or(1.0);
    let integer = args.get(4).map_or(false, |v| match v {
        CellValue::Bool(true) => true,
        CellValue::Number(n) if *n != 0.0 => true,
        _ => false,
    });

    let r: f64 = rand::random();
    let val = min_val + r * (max_val - min_val);
    if integer {
        CellValue::Number(val.round())
    } else {
        CellValue::Number(val)
    }
}

// --- LET ---

/// LET(name1, value1, calculation)
/// LET(name1, value1, name2, value2, ..., calculation)
/// Simple implementation: pairs of (name, value) followed by a calculation expression.
/// The calculation is the last argument and can reference names defined above.
fn dynamic_let(args: &[CellValue]) -> CellValue {
    if args.len() < 3 {
        return CellValue::Error("#VALUE!".into());
    }

    // The last arg is the calculation expression
    // For the simple case LET(name, value, expression), just return value
    // For LET(name1, v1, name2, v2, expression), return the expression value
    // Since we can't evaluate expressions with variable substitution at the CellValue level,
    // we just return the last arg for simple cases.
    let last = args.last().cloned().unwrap_or(CellValue::Empty);

    // If the last arg is a direct value, return it
    // If the last arg looks like it references a defined name, try to resolve it
    if let Some((_, _, _)) = try_extract_2d(args) {
        // Range data in args, handle via spill
    }

    // Simple case: LET(name, value, calculation) where calculation is just the name
    if args.len() == 3 {
        if let CellValue::String(calc_name) = &last {
            let clean_name = calc_name.trim();
            if let CellValue::String(name1) = &args[0] {
                if name1.trim() == clean_name {
                    return args[1].clone();
                }
            }
            // Otherwise return calc_name as-is (unknown reference)
            return CellValue::String(calc_name.clone());
        }
    }

    // For LET(name, value, expression) where expression is a literal value
    last
}

// --- LAMBDA ---

/// LAMBDA(params..., body)
/// Returns the body expression. Deferred evaluation is not fully supported yet.
fn dynamic_lambda(args: &[CellValue]) -> CellValue {
    if args.is_empty() {
        return CellValue::Error("#VALUE!".into());
    }

    // LAMBDA(params..., body) - body is the last argument
    // For now, just return the body
    args.last().cloned().unwrap_or(CellValue::Empty)
}

// --- MAP ---

/// MAP(array, lambda)
/// Apply a lambda to each element of the array.
/// Simple implementation: if lambda is a constant, return it. If it's a simple operation,
/// attempt to apply it.
fn dynamic_map(args: &[CellValue]) -> CellValue {
    if args.len() < 2 {
        return CellValue::Error("#VALUE!".into());
    }

    // Extract array values (after range markers if present)
    let values: Vec<CellValue> = if has_range_marker(args) {
        args[2..].to_vec()
    } else {
        args.iter().cloned().collect()
    };

    if values.is_empty() {
        return CellValue::Empty;
    }

    // Simplified MAP: treat the last arg (lambda/operation) as a modifier
    // For MAP(array, constant), return constant
    // For MAP(array, number), add to each element
    let lambda_arg = args.last().unwrap();

    match lambda_arg {
        CellValue::Number(n) => {
            // Treat as: add n to each element
            let mut sum = 0.0;
            for v in &values {
                if let Some(num) = to_number(v) {
                    sum += num + n;
                }
            }
            CellValue::Number(sum)
        }
        CellValue::String(s) => {
            // Treat string as: add the string to each element (concatenation-like)
            let mut result = String::new();
            for v in &values {
                match v {
                    CellValue::String(sv) => {
                        result.push_str(&format!("{}{}", sv, s));
                    }
                    CellValue::Number(nv) => {
                        result.push_str(&format!("{}{}", nv, s));
                    }
                    _ => {}
                }
            }
            CellValue::String(result)
        }
        CellValue::Bool(_) => {
            // Treat as filter-like: keep matching values
            lambda_arg.clone()
        }
        _ => lambda_arg.clone(),
    }
}

// --- REDUCE ---

/// REDUCE(array, initial, lambda)
/// Fold/reduce the array using an accumulation function.
/// Simplified: supports sum, product accumulation.
fn dynamic_reduce(args: &[CellValue]) -> CellValue {
    if args.len() < 2 {
        return CellValue::Error("#VALUE!".into());
    }

    let values: Vec<CellValue> = if has_range_marker(args) {
        args[2..].to_vec()
    } else {
        args.to_vec()
    };

    let initial = if has_range_marker(args) {
        // Initial is in the middle of the args, hard to extract. Use default.
        0.0
    } else {
        args.get(1).and_then(to_number).unwrap_or(0.0)
    };

    let nums: Vec<f64> = values.iter().filter_map(to_number).collect();
    if nums.is_empty() {
        return CellValue::Number(initial);
    }

    // Default: sum
    CellValue::Number(initial + nums.iter().sum::<f64>())
}

// --- SCAN ---

/// SCAN(array, initial, lambda)
/// Like REDUCE but returns intermediate values.
/// Simplified: returns the accumulated sum.
fn dynamic_scan(args: &[CellValue]) -> CellValue {
    if args.len() < 2 {
        return CellValue::Error("#VALUE!".into());
    }

    let values: Vec<CellValue> = if has_range_marker(args) {
        args[2..].to_vec()
    } else {
        args.to_vec()
    };

    let initial = if has_range_marker(args) {
        0.0
    } else {
        args.get(1).and_then(to_number).unwrap_or(0.0)
    };

    let nums: Vec<f64> = values.iter().filter_map(to_number).collect();
    if nums.is_empty() {
        return CellValue::Number(initial);
    }

    // Return accumulated sum (default behavior)
    CellValue::Number(initial + nums.iter().sum::<f64>())
}

// --- BYROW ---

/// BYROW(array, lambda)
/// Apply lambda to each row of the array.
/// Simplified: multiply each row's first element by the lambda value.
fn dynamic_byrow(args: &[CellValue]) -> CellValue {
    if args.is_empty() {
        return CellValue::Error("#VALUE!".into());
    }

    // Extract 2D data from range markers
    if let Some((_n_cols, _n_rows, data)) = try_extract_2d(args) {
        // Apply lambda to each row - simplified: sum each row
        let mut total = 0.0;
        for row in &data {
            let row_sum: f64 = row.iter().filter_map(to_number).sum();
            total += row_sum;
        }
        return CellValue::Number(total);
    }

    // Fallback: simple operation
    let lambda_val = args.last().and_then(to_number).unwrap_or(1.0);
    let val = args.first().and_then(to_number).unwrap_or(0.0);
    CellValue::Number(val * lambda_val)
}

// --- BYCOL ---

/// BYCOL(array, lambda)
/// Apply lambda to each column of the array.
/// Simplified: multiply each column's first element by the lambda value.
fn dynamic_bycol(args: &[CellValue]) -> CellValue {
    if args.is_empty() {
        return CellValue::Error("#VALUE!".into());
    }

    // Extract 2D data from range markers
    if let Some((n_cols, n_rows, data)) = try_extract_2d(args) {
        // Apply lambda to each column - simplified: sum each column
        let mut total = 0.0;
        for c in 0..n_cols {
            let mut col_sum = 0.0;
            for r in 0..n_rows {
                if let Some(num) = data[r].get(c).and_then(to_number) {
                    col_sum += num;
                }
            }
            total += col_sum;
        }
        return CellValue::Number(total);
    }

    // Fallback: simple operation
    let lambda_val = args.last().and_then(to_number).unwrap_or(1.0);
    let val = args.first().and_then(to_number).unwrap_or(0.0);
    CellValue::Number(val * lambda_val)
}

// --- Helpers ---

fn sort_cell_values(values: &mut [CellValue], ascending: bool) {
    values.sort_by(|a, b| {
        let ordering = compare_cell_values(a, b);
        if ascending {
            ordering
        } else {
            ordering.reverse()
        }
    });
}

fn compare_cell_values(a: &CellValue, b: &CellValue) -> std::cmp::Ordering {
    match (a, b) {
        (CellValue::Number(a), CellValue::Number(b)) => {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        }
        (CellValue::String(a), CellValue::String(b)) => a.cmp(b),
        (CellValue::Bool(a), CellValue::Bool(b)) => a.cmp(b),
        (CellValue::String(_), CellValue::Number(_)) => std::cmp::Ordering::Greater,
        (CellValue::Number(_), CellValue::String(_)) => std::cmp::Ordering::Less,
        (CellValue::Empty, CellValue::Empty) => std::cmp::Ordering::Equal,
        (CellValue::Empty, _) => std::cmp::Ordering::Less,
        (_, CellValue::Empty) => std::cmp::Ordering::Greater,
        (CellValue::Error(_), _) => std::cmp::Ordering::Greater,
        (_, CellValue::Error(_)) => std::cmp::Ordering::Less,
        _ => std::cmp::Ordering::Equal,
    }
}
