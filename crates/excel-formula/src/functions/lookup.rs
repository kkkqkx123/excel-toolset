//! Lookup and reference functions.
//!
//! Range convention for evaluated args:
//! The evaluator expands `AstNode::Range` args into inline CellValues prefixed
//! with dimension markers. A range `B1:C10` (2 cols, 10 rows) becomes:
//!   [Number(-1_000_002.0), Number(10.0), B1, C1, B2, C2, ..., B10, C10]
//!
//! The sentinel is any `Number` < -900_000.0, decoded as:
//!   cols = ((-sentinel) - 1_000_000.0) as usize
//!   rows = next Number as usize
//!   then cols * rows CellValues follow.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

use excel_types::CellValue;

use crate::engine::DataProvider;
use crate::evaluator::{cell_value_to_string, partial_cmp_cell_values, to_number};

// ---------------------------------------------------------------------------
// Registry entry point
// ---------------------------------------------------------------------------

pub fn register(
    registry: &mut HashMap<
        String,
        Arc<dyn Fn(&[CellValue], &dyn DataProvider) -> CellValue + Send + Sync>,
    >,
) {
    registry.insert(
        "VLOOKUP".into(),
        Arc::new(|args, provider| lookup_vlookup(args, provider)),
    );
    registry.insert(
        "XLOOKUP".into(),
        Arc::new(|args, provider| lookup_xlookup(args, provider)),
    );
    registry.insert(
        "HLOOKUP".into(),
        Arc::new(|args, provider| lookup_hlookup(args, provider)),
    );
    registry.insert(
        "INDEX".into(),
        Arc::new(|args, provider| lookup_index(args, provider)),
    );
    registry.insert(
        "MATCH".into(),
        Arc::new(|args, provider| lookup_match(args, provider)),
    );
    registry.insert(
        "OFFSET".into(),
        Arc::new(|args, provider| lookup_offset(args, provider)),
    );
    registry.insert(
        "INDIRECT".into(),
        Arc::new(|args, provider| lookup_indirect(args, provider)),
    );
    registry.insert("ROW".into(), Arc::new(|args, _p| lookup_row(args)));
    registry.insert("COLUMN".into(), Arc::new(|args, _p| lookup_column(args)));
    registry.insert("ROWS".into(), Arc::new(|args, _p| lookup_rows(args)));
    registry.insert("COLUMNS".into(), Arc::new(|args, _p| lookup_columns(args)));
    registry.insert("CHOOSE".into(), Arc::new(|args, _p| lookup_choose(args)));
    registry.insert("ADDRESS".into(), Arc::new(|args, _p| lookup_address(args)));
}

// ---------------------------------------------------------------------------
// Range-marker helpers
// ---------------------------------------------------------------------------

/// Sentinel: numbers below this threshold are range column-count markers.
const RANGE_MARKER_THRESHOLD: f64 = -900_000.0;
const RANGE_MARKER_OFFSET: f64 = 1_000_000.0;

/// Check whether a CellValue is a range dimension sentinel.
#[allow(dead_code)]
fn is_range_marker(val: &CellValue) -> bool {
    matches!(val, CellValue::Number(n) if *n < RANGE_MARKER_THRESHOLD)
}

/// Decode the column count from a sentinel marker value.
fn decode_cols(marker: f64) -> usize {
    ((-marker) - RANGE_MARKER_OFFSET) as usize
}

/// Try to consume a range marker starting at `args[start]`.
///
/// On success returns `Some((cols, rows, data_end))` where `data_end` is the
/// index of the first arg *after* the expanded range values.
fn consume_range_marker(args: &[CellValue], start: usize) -> Option<(usize, usize, usize)> {
    if start >= args.len() {
        return None;
    }
    if let CellValue::Number(n) = &args[start] {
        if *n < RANGE_MARKER_THRESHOLD {
            let cols = decode_cols(*n);
            if start + 1 >= args.len() {
                return None;
            }
            let rows = match &args[start + 1] {
                CellValue::Number(r) => *r as usize,
                _ => return None,
            };
            let data_end = start + 2 + cols * rows;
            if data_end > args.len() {
                return None;
            }
            return Some((cols, rows, data_end));
        }
    }
    None
}

/// Build a 2D table ```table[row][col]``` from flat range data.
fn build_2d_table(data: &[CellValue], cols: usize, rows: usize) -> Vec<Vec<CellValue>> {
    let mut table = Vec::with_capacity(rows);
    for r in 0..rows {
        let start = r * cols;
        table.push(data[start..start + cols].to_vec());
    }
    table
}

/// Extract the first column from a 2D table.
fn extract_column(table: &[Vec<CellValue>], col_idx: usize) -> Vec<&CellValue> {
    table.iter().map(|row| &row[col_idx]).collect()
}

// ---------------------------------------------------------------------------
// Exact / approximate match utilities
// ---------------------------------------------------------------------------

/// Excel-style equality for lookup exact match.
///
/// Strings are compared case-insensitively. Cross-type comparisons
/// (e.g., number vs string) always return false.
fn lookup_values_equal(a: &CellValue, b: &CellValue) -> bool {
    match (a, b) {
        (CellValue::Number(x), CellValue::Number(y)) => x == y,
        (CellValue::String(x), CellValue::String(y)) => x.to_lowercase() == y.to_lowercase(),
        (CellValue::Bool(x), CellValue::Bool(y)) => x == y,
        (CellValue::Empty, CellValue::Empty) => true,
        _ => false,
    }
}

/// Perform exact match search in a slice of CellValues.
///
/// Returns `Some(index)` if found, `None` otherwise.
fn exact_match(lookup_value: &CellValue, haystack: &[&CellValue]) -> Option<usize> {
    haystack
        .iter()
        .position(|v| lookup_values_equal(v, lookup_value))
}

/// Direction for approximate match search.
enum ApproxMode {
    /// Find the largest value <= lookup (ascending data).
    LargestLe,
    /// Find the smallest value >= lookup (descending data).
    SmallestGe,
}

/// Perform approximate match search in a sorted slice of CellValues.
///
/// `mode` controls the direction: `LargestLe` for VLOOKUP/HLOOKUP/type-1 MATCH,
/// `SmallestGe` for type-(-1) MATCH.
fn approximate_match(
    lookup_value: &CellValue,
    haystack: &[&CellValue],
    mode: ApproxMode,
) -> Option<usize> {
    match mode {
        ApproxMode::SmallestGe => {
            // match_type = -1: find smallest value >= lookup_value
            for (i, v) in haystack.iter().enumerate() {
                if matches!(
                    partial_cmp_cell_values(v, lookup_value),
                    Some(Ordering::Greater | Ordering::Equal)
                ) {
                    return Some(i);
                }
            }
            None
        }
        ApproxMode::LargestLe => {
            // match_type = 1: find largest value <= lookup_value
            let mut best: Option<usize> = None;
            for (i, v) in haystack.iter().enumerate() {
                if matches!(
                    partial_cmp_cell_values(v, lookup_value),
                    Some(Ordering::Less | Ordering::Equal)
                ) {
                    best = Some(i);
                } else {
                    break;
                }
            }
            best
        }
    }
}

// ---------------------------------------------------------------------------
// A1 notation parsing (for INDIRECT)
// ---------------------------------------------------------------------------

/// Parse a column-letter string (e.g. "A", "AB") into a 0-based column index.
fn col_letters_to_index(s: &str) -> Option<u32> {
    if s.is_empty() {
        return None;
    }
    let mut col = 0u32;
    for ch in s.bytes() {
        if !ch.is_ascii_uppercase() {
            return None;
        }
        col = col.checked_mul(26)?.checked_add((ch - b'A') as u32 + 1)?;
    }
    col.checked_sub(1)
}

/// Parse an A1-style reference like "A1" or "Sheet1!A1" into a (sheet, row, col).
///
/// Returns `(sheet_name, row_0based, col_0based)` on success.
fn parse_a1_ref(text: &str) -> Option<(String, u32, u32)> {
    let s = text.trim();

    // Split sheet name if present: "Sheet1!A1"
    let (sheet, cell_part) = match s.find('!') {
        Some(idx) => {
            let raw_sheet = &s[..idx];
            // Remove surrounding single quotes if any
            let sheet_name = raw_sheet
                .strip_prefix('\'')
                .and_then(|t| t.strip_suffix('\''))
                .unwrap_or(raw_sheet);
            (sheet_name.to_string(), &s[idx + 1..])
        }
        None => (String::new(), s),
    };

    // Parse the column letters
    let col_end = cell_part
        .find(|c: char| c.is_ascii_digit())
        .unwrap_or(cell_part.len());
    let col_str = &cell_part[..col_end];
    let row_str = &cell_part[col_end..];

    let col = col_letters_to_index(col_str)?;
    let row: u32 = row_str.parse().ok()?;
    if row == 0 {
        return None;
    }

    Some((sheet, row - 1, col))
}

// ---------------------------------------------------------------------------
// VLOOKUP(lookup_value, table_array, col_index_num, [range_lookup])
// ---------------------------------------------------------------------------

fn lookup_vlookup(args: &[CellValue], _provider: &dyn DataProvider) -> CellValue {
    if args.len() < 2 {
        return CellValue::Error("#VALUE!".into());
    }

    let lookup_value = &args[0];

    // --- Parse table and remaining params ---
    let (table, col_index, range_lookup) =
        if let Some((cols, rows, range_end)) = consume_range_marker(args, 1) {
            // Range marker present: remaining args start after the expanded range.
            let table = build_2d_table(&args[1 + 2..range_end], cols, rows);
            let remaining = &args[range_end..];
            let col_index = remaining.first().and_then(to_number).unwrap_or(1.0) as usize;
            let range_lookup = remaining
                .get(1)
                .map_or(true, |v| !matches!(v, CellValue::Bool(false)));
            (table, col_index, range_lookup)
        } else {
            // No range marker: use inline args as single-column table.
            match parse_inline_vlookup_args(args) {
                Some((table, ci, rl)) => (table, ci, rl),
                None => return CellValue::Error("#VALUE!".into()),
            }
        };

    if col_index == 0 {
        return CellValue::Error("#VALUE!".into());
    }
    if table.is_empty() {
        return CellValue::Error("#N/A".into());
    }
    let n_cols = table[0].len();
    if col_index > n_cols {
        return CellValue::Error("#REF!".into());
    }

    let col_idx = col_index - 1;
    let search_col: Vec<&CellValue> = extract_column(&table, 0);

    if range_lookup {
        // Approximate: data must be sorted ascending.
        match approximate_match(lookup_value, &search_col, ApproxMode::LargestLe) {
            Some(r) => {
                if r < table.len() {
                    table[r][col_idx].clone()
                } else {
                    CellValue::Error("#N/A".into())
                }
            }
            None => CellValue::Error("#N/A".into()),
        }
    } else {
        // Exact match
        match exact_match(lookup_value, &search_col) {
            Some(r) => table[r][col_idx].clone(),
            None => CellValue::Error("#N/A".into()),
        }
    }
}

/// Parse inline args for VLOOKUP when no range marker is present.
///
/// Inline pattern: `args[0]=lookup, args[1..n-2/1]=table cells, ..., col_index, [range_lookup]`.
fn parse_inline_vlookup_args(args: &[CellValue]) -> Option<(Vec<Vec<CellValue>>, usize, bool)> {
    if args.len() < 3 {
        return None;
    }

    // Last arg is range_lookup if it's a Bool
    let has_range_lookup = matches!(args.last()?, CellValue::Bool(_));
    let param_count = if has_range_lookup { 2 } else { 1 };

    if args.len() < 2 + param_count {
        return None;
    }

    let table_vals_end = args.len() - param_count;
    let col_index = args[table_vals_end].clone();
    let col_index = to_number(&col_index)? as usize;

    let range_lookup = if has_range_lookup {
        match args.last().expect("checked") {
            CellValue::Bool(b) => *b,
            _ => true,
        }
    } else {
        true
    };

    let table_data = &args[1..table_vals_end];
    let table: Vec<Vec<CellValue>> = table_data.iter().map(|v| vec![v.clone()]).collect();

    Some((table, col_index, range_lookup))
}

// ---------------------------------------------------------------------------
// XLOOKUP(lookup_value, lookup_array, return_array, [if_not_found], [match_mode], [search_mode])
// ---------------------------------------------------------------------------

fn lookup_xlookup(args: &[CellValue], _provider: &dyn DataProvider) -> CellValue {
    if args.len() < 2 {
        return CellValue::Error("#VALUE!".into());
    }

    let lookup_value = &args[0];

    // Consume lookup_array range marker (if present)
    let (lookup_array, rest) = if let Some((_cols, _rows, end)) = consume_range_marker(args, 1) {
        let flat = &args[1 + 2..end];
        (flat.to_vec(), &args[end..])
    } else {
        // No range marker: args[1] is the only lookup_array element
        (args[1..].to_vec(), &[] as &[CellValue])
    };

    // Consume return_array range marker (if present)
    let rest = if rest.is_empty() {
        &args[1 + lookup_array.len()..]
    } else {
        rest
    };

    let (return_array, rest) = if !rest.is_empty() {
        if let Some((_cols, _rows, end)) = consume_range_marker(rest, 0) {
            let flat = &rest[2..end];
            (flat.to_vec(), &rest[end..])
        } else {
            (rest.to_vec(), &[] as &[CellValue])
        }
    } else {
        (vec![], rest)
    };

    // Remaining params
    let if_not_found = rest.first().cloned();
    let match_mode = rest
        .get(if if_not_found.is_some() { 1 } else { 0 })
        .and_then(to_number)
        .unwrap_or(0.0) as i32;
    // search_mode: 1=first-to-last, -1=last-to-first, 2=binary-asc, -2=binary-desc
    let search_mode = rest
        .get(if if_not_found.is_some() { 2 } else { 1 })
        .and_then(to_number)
        .unwrap_or(1.0) as i32;

    if lookup_array.is_empty() {
        return if_not_found.unwrap_or(CellValue::Error("#N/A".into()));
    }

    // Build the search order
    let indices = match search_mode {
        -1 => (0..lookup_array.len()).rev().collect::<Vec<_>>(),
        _ => (0..lookup_array.len()).collect::<Vec<_>>(),
    };

    let search_refs: Vec<&CellValue> = lookup_array.iter().collect();

    match match_mode {
        -1 => {
            // Exact match, or next smaller item
            match exact_match(lookup_value, &search_refs) {
                Some(idx) => {
                    if idx < return_array.len() {
                        return_array[idx].clone()
                    } else {
                        CellValue::Error("#N/A".into())
                    }
                }
                None => {
                    // Try approximate (largest <= lookup)
                    let sorted_refs: Vec<&CellValue> =
                        indices.iter().map(|&i| search_refs[i]).collect();
                    match approximate_match(lookup_value, &sorted_refs, ApproxMode::LargestLe) {
                        Some(idx) => {
                            if idx < return_array.len() {
                                return_array[indices[idx]].clone()
                            } else {
                                CellValue::Error("#N/A".into())
                            }
                        }
                        None => if_not_found.unwrap_or(CellValue::Error("#N/A".into())),
                    }
                }
            }
        }
        1 => {
            // Exact match, or next larger item
            match exact_match(lookup_value, &search_refs) {
                Some(idx) => {
                    if idx < return_array.len() {
                        return_array[idx].clone()
                    } else {
                        CellValue::Error("#N/A".into())
                    }
                }
                None => {
                    let sorted_refs: Vec<&CellValue> =
                        indices.iter().map(|&i| search_refs[i]).collect();
                    match approximate_match(lookup_value, &sorted_refs, ApproxMode::SmallestGe) {
                        Some(idx) => {
                            if idx < return_array.len() {
                                return_array[indices[idx]].clone()
                            } else {
                                CellValue::Error("#N/A".into())
                            }
                        }
                        None => if_not_found.unwrap_or(CellValue::Error("#N/A".into())),
                    }
                }
            }
        }
        2 => {
            // Wildcard match (simplified: treat as exact match)
            match exact_match(lookup_value, &search_refs) {
                Some(idx) if idx < return_array.len() => return_array[idx].clone(),
                _ => if_not_found.unwrap_or(CellValue::Error("#N/A".into())),
            }
        }
        _ => {
            // Default: exact match (match_mode = 0)
            for &i in &indices {
                if lookup_values_equal(&lookup_array[i], lookup_value) {
                    return if i < return_array.len() {
                        return_array[i].clone()
                    } else {
                        CellValue::Error("#N/A".into())
                    };
                }
            }
            if_not_found.unwrap_or(CellValue::Error("#N/A".into()))
        }
    }
}

// ---------------------------------------------------------------------------
// HLOOKUP(lookup_value, table_array, row_index_num, [range_lookup])
// ---------------------------------------------------------------------------

fn lookup_hlookup(args: &[CellValue], _provider: &dyn DataProvider) -> CellValue {
    if args.len() < 2 {
        return CellValue::Error("#VALUE!".into());
    }

    let lookup_value = &args[0];

    let (table, row_index, range_lookup) =
        if let Some((cols, rows, range_end)) = consume_range_marker(args, 1) {
            let table = build_2d_table(&args[1 + 2..range_end], cols, rows);
            let remaining = &args[range_end..];
            let row_index = remaining.first().and_then(to_number).unwrap_or(1.0) as usize;
            let range_lookup = remaining
                .get(1)
                .map_or(true, |v| !matches!(v, CellValue::Bool(false)));
            (table, row_index, range_lookup)
        } else {
            // Inline args: treat as single-row table
            if args.len() < 3 {
                return CellValue::Error("#VALUE!".into());
            }
            let has_range_lookup = matches!(args.last(), Some(CellValue::Bool(_)));
            let param_count = if has_range_lookup { 2 } else { 1 };
            if args.len() < 2 + param_count {
                return CellValue::Error("#VALUE!".into());
            }
            let table_end = args.len() - param_count;
            let row_index = to_number(&args[table_end]).unwrap_or(1.0) as usize;
            let range_lookup = if has_range_lookup {
                match args.last().expect("checked") {
                    CellValue::Bool(b) => *b,
                    _ => true,
                }
            } else {
                true
            };
            // Treat as single-row table: each value is a column
            let table_data = &args[1..table_end];
            let table: Vec<Vec<CellValue>> = vec![table_data.to_vec()];
            (table, row_index, range_lookup)
        };

    if row_index == 0 {
        return CellValue::Error("#VALUE!".into());
    }
    if table.is_empty() || table[0].is_empty() {
        return CellValue::Error("#N/A".into());
    }
    if row_index > table.len() {
        return CellValue::Error("#REF!".into());
    }

    let row_idx = row_index - 1;
    // Search the first row (table[0]) horizontally
    let header: Vec<&CellValue> = table[0].iter().collect();

    if range_lookup {
        match approximate_match(lookup_value, &header, ApproxMode::LargestLe) {
            Some(col) => {
                if col < table[row_idx].len() {
                    table[row_idx][col].clone()
                } else {
                    CellValue::Error("#N/A".into())
                }
            }
            None => CellValue::Error("#N/A".into()),
        }
    } else {
        match exact_match(lookup_value, &header) {
            Some(col) => {
                if col < table[row_idx].len() {
                    table[row_idx][col].clone()
                } else {
                    CellValue::Error("#N/A".into())
                }
            }
            None => CellValue::Error("#N/A".into()),
        }
    }
}

// ---------------------------------------------------------------------------
// INDEX(array, row_num, [col_num])
// ---------------------------------------------------------------------------

fn lookup_index(args: &[CellValue], _provider: &dyn DataProvider) -> CellValue {
    if args.len() < 2 {
        return CellValue::Error("#VALUE!".into());
    }

    let row_num = args.get(1).and_then(to_number);
    if row_num.is_none() {
        return CellValue::Error("#VALUE!".into());
    }
    let row_num = row_num.unwrap() as usize;
    if row_num == 0 {
        return CellValue::Error("#VALUE!".into());
    }

    let row_idx = row_num - 1;

    // Check if args[0] is a range marker (2D table)
    if let Some((cols, rows, range_end)) = consume_range_marker(args, 0) {
        let table = build_2d_table(&args[2..range_end], cols, rows);
        let remaining = &args[range_end..];
        // Parse row_num and col_num from remaining args (they were passed after range)
        let row_num = remaining.first().and_then(to_number).unwrap_or(1.0) as usize;
        let col_num = remaining.get(1).and_then(to_number);

        if row_num == 0 || row_num > table.len() {
            return CellValue::Error("#REF!".into());
        }
        let row_idx = row_num - 1;
        if let Some(cn) = col_num {
            let cn = cn as usize;
            if cn == 0 || cn > table[row_idx].len() {
                return CellValue::Error("#REF!".into());
            }
            table[row_idx][cn - 1].clone()
        } else {
            // Return whole row: we can only return the first cell
            table[row_idx][0].clone()
        }
    } else if let Some(col_num) = args.get(2).and_then(to_number) {
        // Inline array with row_num and col_num
        let col_num = col_num as usize;
        if col_num == 0 {
            return CellValue::Error("#VALUE!".into());
        }
        // args[0] is the single array value; with a col_num we need 2D data.
        // Only row=1, col=1 is valid for a single-value inline array.
        if row_num == 1 && col_num == 1 {
            args[0].clone()
        } else {
            CellValue::Error("#REF!".into())
        }
    } else {
        // INDEX(array, row_num) — single column, inline/single-value array.
        // Without a range marker, args[0] is the only array element;
        // args[1] is row_num. Only row 1 is valid.
        if row_num == 1 {
            args[0].clone()
        } else {
            CellValue::Error("#REF!".into())
        }
    }
}

// ---------------------------------------------------------------------------
// MATCH(lookup_value, lookup_array, [match_type])
// ---------------------------------------------------------------------------

fn lookup_match(args: &[CellValue], _provider: &dyn DataProvider) -> CellValue {
    if args.len() < 2 {
        return CellValue::Error("#VALUE!".into());
    }

    let lookup_value = &args[0];

    // Consume range marker (if present) and determine match_type from remainder.
    let (array, remainder) = if let Some((_cols, _rows, end)) = consume_range_marker(args, 1) {
        let flat = &args[1 + 2..end];
        (flat.to_vec(), &args[end..])
    } else {
        // No range marker: args[1..] are the array values.
        // Determine where the array ends and match_type begins.
        let end = if args.len() > 2 && to_number(&args[args.len() - 1]).is_some() {
            args.len() - 1
        } else {
            args.len()
        };
        (args[1..end].to_vec(), &args[end..])
    };

    let match_type = remainder.first().and_then(to_number).unwrap_or(1.0) as i32;

    if array.is_empty() {
        return CellValue::Error("#N/A".into());
    }

    let refs: Vec<&CellValue> = array.iter().collect();

    match match_type {
        0 => {
            // Exact match
            match exact_match(lookup_value, &refs) {
                Some(idx) => CellValue::Number((idx + 1) as f64),
                None => CellValue::Error("#N/A".into()),
            }
        }
        -1 => {
            // Exact or next larger; array must be sorted descending
            match approximate_match(lookup_value, &refs, ApproxMode::SmallestGe) {
                Some(idx) => CellValue::Number((idx + 1) as f64),
                None => CellValue::Error("#N/A".into()),
            }
        }
        _ => {
            // match_type = 1: exact or next smaller; array must be sorted ascending
            match approximate_match(lookup_value, &refs, ApproxMode::LargestLe) {
                Some(idx) => CellValue::Number((idx + 1) as f64),
                None => CellValue::Error("#N/A".into()),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// OFFSET(reference, rows, cols, [height], [width])
// ---------------------------------------------------------------------------

fn lookup_offset(args: &[CellValue], _provider: &dyn DataProvider) -> CellValue {
    // OFFSET requires knowledge of the original cell coordinates, which are
    // not available from evaluated CellValues alone. When a concrete cell-
    // coordinate-aware call site is added this function can be made to use
    // the DataProvider.
    let _ref = args.first();
    let _rows = args.get(1).and_then(to_number).unwrap_or(0.0);
    let _cols = args.get(2).and_then(to_number).unwrap_or(0.0);
    CellValue::Error("#REF!".into())
}

// ---------------------------------------------------------------------------
// INDIRECT(ref_text, [a1])
// ---------------------------------------------------------------------------

fn lookup_indirect(args: &[CellValue], provider: &dyn DataProvider) -> CellValue {
    let ref_text = args
        .first()
        .map(|v| cell_value_to_string(v))
        .unwrap_or_default();
    // A1 flag: if FALSE, use R1C1; not yet supported.
    let _a1 = args
        .get(1)
        .map_or(true, |v| !matches!(v, CellValue::Bool(false)));

    if ref_text.is_empty() {
        return CellValue::Error("#REF!".into());
    }

    // Parse A1 notation
    let (sheet, row, col) = match parse_a1_ref(&ref_text) {
        Some(t) => t,
        None => return CellValue::Error("#REF!".into()),
    };

    let sheet_name = if sheet.is_empty() {
        // No sheet specified — the provider may or may not handle this.
        // We need a sheet name to call get_cell.
        return CellValue::Error("#REF!".into());
    } else {
        sheet
    };

    provider
        .get_cell(&sheet_name, row, col)
        .unwrap_or(CellValue::Error("#REF!".into()))
}

// ---------------------------------------------------------------------------
// ROW([reference])
// ---------------------------------------------------------------------------

fn lookup_row(_args: &[CellValue]) -> CellValue {
    // Without cell coordinate context, return 1.
    CellValue::Number(1.0)
}

// ---------------------------------------------------------------------------
// COLUMN([reference])
// ---------------------------------------------------------------------------

fn lookup_column(_args: &[CellValue]) -> CellValue {
    // Without cell coordinate context, return 1.
    CellValue::Number(1.0)
}

// ---------------------------------------------------------------------------
// ROWS(array)
// ---------------------------------------------------------------------------

fn lookup_rows(args: &[CellValue]) -> CellValue {
    if let Some((_cols, rows, _end)) = consume_range_marker(args, 0) {
        return CellValue::Number(rows as f64);
    }
    // No range marker: count from args. If args are a flat list, rows = 1.
    // For inline 2D arrays, we cannot determine dimensions.
    match args.len() {
        0 => CellValue::Error("#VALUE!".into()),
        _ => CellValue::Number(1.0),
    }
}

// ---------------------------------------------------------------------------
// COLUMNS(array)
// ---------------------------------------------------------------------------

fn lookup_columns(args: &[CellValue]) -> CellValue {
    if let Some((cols, _rows, _end)) = consume_range_marker(args, 0) {
        return CellValue::Number(cols as f64);
    }
    // No range marker: treat flat args as a single row, count columns = len(args).
    CellValue::Number(args.len() as f64)
}

// ---------------------------------------------------------------------------
// CHOOSE(index_num, value1, [value2], ...)
// ---------------------------------------------------------------------------

fn lookup_choose(args: &[CellValue]) -> CellValue {
    if args.is_empty() {
        return CellValue::Error("#VALUE!".into());
    }

    let idx = match to_number(&args[0]) {
        Some(n) if n >= 1.0 => n as usize,
        _ => return CellValue::Error("#VALUE!".into()),
    };

    if idx >= args.len() {
        CellValue::Error("#VALUE!".into())
    } else {
        args[idx].clone()
    }
}

// ---------------------------------------------------------------------------
// ADDRESS(row_num, column_num, [abs_num], [a1], [sheet_text])
// ---------------------------------------------------------------------------

fn lookup_address(args: &[CellValue]) -> CellValue {
    let row = args.first().and_then(to_number).unwrap_or(1.0) as u32;
    let col = args.get(1).and_then(to_number).unwrap_or(1.0) as u32;
    let abs_num = args.get(2).and_then(to_number).unwrap_or(1.0) as u32;
    let _a1 = args
        .get(3)
        .map_or(true, |v| !matches!(v, CellValue::Bool(false)));
    let sheet = args.get(4).map(cell_value_to_string);

    if row == 0 || col == 0 {
        return CellValue::Error("#VALUE!".into());
    }

    let col_str = col_index_to_letters(col.saturating_sub(1));

    let cell = match abs_num {
        1 => format!("${}${}", col_str, row), // Absolute
        2 => format!("{}${}", col_str, row),  // Row absolute
        3 => format!("${}{}", col_str, row),  // Col absolute
        4 => format!("{}{}", col_str, row),   // Relative
        _ => format!("${}${}", col_str, row),
    };

    let result = if let Some(s) = sheet {
        format!("{}!{}", s, cell)
    } else {
        cell
    };

    CellValue::String(result)
}

/// Convert a 0-based column index to Excel column letters (A, B, ..., Z, AA, ...).
fn col_index_to_letters(mut col: u32) -> String {
    let mut result = String::new();
    loop {
        let remainder = (col % 26) as u8;
        result.push((b'A' + remainder) as char);
        if col < 26 {
            break;
        }
        col = col / 26 - 1;
    }
    result.chars().rev().collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Helpers ---

    fn num(n: f64) -> CellValue {
        CellValue::Number(n)
    }
    fn txt(s: &str) -> CellValue {
        CellValue::String(s.into())
    }
    fn bool_val(b: bool) -> CellValue {
        CellValue::Bool(b)
    }
    fn err(e: &str) -> CellValue {
        CellValue::Error(e.into())
    }

    // Build range-marker prefix for test data: [sentinel, rows, ...cells]
    fn range_marker(cols: usize, rows: usize) -> Vec<CellValue> {
        vec![
            CellValue::Number(-(cols as f64 + 1_000_000.0)),
            CellValue::Number(rows as f64),
        ]
    }

    // --- Range marker tests ---

    #[test]
    fn test_consume_range_marker_valid() {
        let mut args = range_marker(2, 3);
        // Add 6 cell values (2 cols * 3 rows)
        for i in 0..6 {
            args.push(num(i as f64));
        }
        // Append extra params after the range
        args.push(num(2.0)); // col_index for VLOOKUP

        assert!(is_range_marker(&args[0]));
        let result = consume_range_marker(&args, 0);
        assert!(result.is_some());
        let (cols, rows, end) = result.unwrap();
        assert_eq!(cols, 2);
        assert_eq!(rows, 3);
        assert_eq!(end, 8); // 2 (marker+rows) + 6 cells
        assert_eq!(args[end], num(2.0)); // extra param preserved after end
    }

    #[test]
    fn test_consume_range_marker_not_found() {
        let args = vec![num(42.0), txt("hello")];
        assert!(consume_range_marker(&args, 0).is_none());
    }

    #[test]
    fn test_col_letters() {
        assert_eq!(col_index_to_letters(0), "A");
        assert_eq!(col_index_to_letters(25), "Z");
        assert_eq!(col_index_to_letters(26), "AA");
        assert_eq!(col_index_to_letters(27), "AB");
        assert_eq!(col_index_to_letters(701), "ZZ");
    }

    #[test]
    fn test_parse_a1_ref() {
        assert_eq!(parse_a1_ref("A1"), Some((String::new(), 0, 0)));
        assert_eq!(parse_a1_ref("B2"), Some((String::new(), 1, 1)));
        assert_eq!(parse_a1_ref("ZZ10"), Some((String::new(), 9, 701)));
        assert_eq!(parse_a1_ref("Sheet1!A1"), Some(("Sheet1".into(), 0, 0)));
        assert_eq!(
            parse_a1_ref("'My Sheet'!B3"),
            Some(("My Sheet".into(), 2, 1))
        );
        assert_eq!(parse_a1_ref(""), None);
        assert_eq!(parse_a1_ref("1A"), None);
    }

    // --- VLOOKUP tests ---

    #[test]
    fn test_vlookup_exact_match_with_range() {
        // Simulate VLOOKUP(42, A1:B3, 2, FALSE)
        // Range: 3 rows, 2 cols: [(10,"a"), (20,"b"), (42,"target")]
        let mut args = vec![num(42.0)]; // lookup_value
        args.extend(range_marker(2, 3));
        args.extend(vec![
            num(10.0),
            txt("a"), // row 1
            num(20.0),
            txt("b"), // row 2
            num(42.0),
            txt("target"), // row 3
        ]);
        args.push(num(2.0)); // col_index
        args.push(bool_val(false)); // exact
        let dummy_provider = InMemoryDataProvider::new_shared();
        let result = lookup_vlookup(&args, &*dummy_provider);
        assert_eq!(result, txt("target"));
    }

    #[test]
    fn test_vlookup_not_found() {
        let mut args = vec![num(99.0)];
        args.extend(range_marker(1, 2));
        args.extend(vec![num(10.0), num(20.0)]);
        args.push(num(1.0));
        args.push(bool_val(false));
        let dummy_provider = InMemoryDataProvider::new_shared();
        let result = lookup_vlookup(&args, &*dummy_provider);
        assert_eq!(result, err("#N/A"));
    }

    #[test]
    fn test_vlookup_approximate_match() {
        // Sorted ascending: 10, 20, 30. Lookup 25 -> returns 20's row value.
        let mut args = vec![num(25.0)];
        args.extend(range_marker(2, 3));
        args.extend(vec![
            num(10.0),
            txt("ten"),
            num(20.0),
            txt("twenty"),
            num(30.0),
            txt("thirty"),
        ]);
        args.push(num(2.0)); // col_index
        // No range_lookup = approximate (default TRUE)
        let dummy_provider = InMemoryDataProvider::new_shared();
        let result = lookup_vlookup(&args, &*dummy_provider);
        assert_eq!(result, txt("twenty"));
    }

    #[test]
    fn test_vlookup_col_out_of_range() {
        let mut args = vec![num(1.0)];
        args.extend(range_marker(1, 1));
        args.extend(vec![num(10.0)]);
        args.push(num(5.0)); // col_index 5 exceeds 1 column
        args.push(bool_val(false));
        let dummy_provider = InMemoryDataProvider::new_shared();
        let result = lookup_vlookup(&args, &*dummy_provider);
        assert_eq!(result, err("#REF!"));
    }

    #[test]
    fn test_vlookup_insufficient_args() {
        let dummy_provider = InMemoryDataProvider::new_shared();
        assert_eq!(
            lookup_vlookup(&[num(1.0)], &*dummy_provider),
            err("#VALUE!")
        );
    }

    // --- HLOOKUP tests ---

    #[test]
    fn test_hlookup_exact_match_with_range() {
        // Table: header row [10, 20, 42], data row [a, b, target]
        // HLOOKUP(42, table, 2, FALSE)
        let mut args = vec![num(42.0)];
        args.extend(range_marker(3, 2));
        args.extend(vec![
            num(10.0),
            num(20.0),
            num(42.0), // row 1 (header)
            txt("a"),
            txt("b"),
            txt("target"), // row 2 (data)
        ]);
        args.push(num(2.0)); // row_index
        args.push(bool_val(false)); // exact
        let dummy_provider = InMemoryDataProvider::new_shared();
        let result = lookup_hlookup(&args, &*dummy_provider);
        assert_eq!(result, txt("target"));
    }

    #[test]
    fn test_hlookup_not_found() {
        let mut args = vec![num(99.0)];
        args.extend(range_marker(2, 1));
        args.extend(vec![num(10.0), num(20.0)]);
        args.push(num(1.0));
        args.push(bool_val(false));
        let dummy_provider = InMemoryDataProvider::new_shared();
        let result = lookup_hlookup(&args, &*dummy_provider);
        assert_eq!(result, err("#N/A"));
    }

    // --- MATCH tests ---

    #[test]
    fn test_match_exact() {
        let mut args = vec![txt("b")];
        args.extend(range_marker(1, 3));
        args.extend(vec![txt("a"), txt("b"), txt("c")]);
        args.push(num(0.0)); // match_type exact
        let dummy_provider = InMemoryDataProvider::new_shared();
        let result = lookup_match(&args, &*dummy_provider);
        assert_eq!(result, num(2.0)); // 1-based position
    }

    #[test]
    fn test_match_exact_not_found() {
        let mut args = vec![txt("x")];
        args.extend(range_marker(1, 3));
        args.extend(vec![txt("a"), txt("b"), txt("c")]);
        args.push(num(0.0));
        let dummy_provider = InMemoryDataProvider::new_shared();
        let result = lookup_match(&args, &*dummy_provider);
        assert_eq!(result, err("#N/A"));
    }

    #[test]
    fn test_match_less_than() {
        // match_type=1: largest value <= lookup. Sorted ascending: 10, 20, 30. Lookup 25 -> 20 at pos 2.
        let mut args = vec![num(25.0)];
        args.extend(range_marker(1, 3));
        args.extend(vec![num(10.0), num(20.0), num(30.0)]);
        args.push(num(1.0)); // default match_type = less than
        let dummy_provider = InMemoryDataProvider::new_shared();
        let result = lookup_match(&args, &*dummy_provider);
        assert_eq!(result, num(2.0));
    }

    // --- INDEX tests ---

    #[test]
    fn test_index_range_2d() {
        // INDEX(A1:B3, 2, 2)
        let mut args = Vec::new();
        args.extend(range_marker(2, 3));
        args.extend(vec![
            num(1.0),
            txt("x"), // row 1
            num(2.0),
            txt("y"), // row 2
            num(3.0),
            txt("z"), // row 3
        ]);
        args.push(num(2.0)); // row_num
        args.push(num(2.0)); // col_num
        let dummy_provider = InMemoryDataProvider::new_shared();
        let result = lookup_index(&args, &*dummy_provider);
        assert_eq!(result, txt("y"));
    }

    #[test]
    fn test_index_inline_1d() {
        // INDEX({10}, 1) -> 10 (inline array flattened to single value)
        let dummy_provider = InMemoryDataProvider::new_shared();
        let result = lookup_index(
            &[num(10.0), num(1.0)], // args[0]=array_value, args[1]=row_num
            &*dummy_provider,
        );
        assert_eq!(result, num(10.0));

        // INDEX({10}, 2) -> #REF! (row 2 is out of bounds)
        let result2 = lookup_index(&[num(10.0), num(2.0)], &*dummy_provider);
        assert_eq!(result2, err("#REF!"));
    }

    #[test]
    fn test_index_row_out_of_range() {
        let mut args = Vec::new();
        args.extend(range_marker(1, 1));
        args.extend(vec![num(42.0)]);
        args.push(num(5.0)); // row_num 5 exceeds 1 row
        args.push(num(1.0)); // col_num
        let dummy_provider = InMemoryDataProvider::new_shared();
        let result = lookup_index(&args, &*dummy_provider);
        assert_eq!(result, err("#REF!"));
    }

    // --- XLOOKUP tests ---

    #[test]
    fn test_xlookup_exact_match() {
        // XLOOKUP("b", lookup_array, return_array)
        let mut args = vec![txt("b")];
        args.extend(range_marker(1, 3));
        args.extend(vec![txt("a"), txt("b"), txt("c")]); // lookup_array
        args.extend(range_marker(1, 3));
        args.extend(vec![num(1.0), num(2.0), num(3.0)]); // return_array
        let dummy_provider = InMemoryDataProvider::new_shared();
        let result = lookup_xlookup(&args, &*dummy_provider);
        assert_eq!(result, num(2.0));
    }

    #[test]
    fn test_xlookup_not_found_with_default() {
        let mut args = vec![txt("z")];
        args.extend(range_marker(1, 2));
        args.extend(vec![txt("a"), txt("b")]); // lookup_array
        args.extend(range_marker(1, 2));
        args.extend(vec![num(10.0), num(20.0)]); // return_array
        args.push(txt("not found")); // if_not_found
        let dummy_provider = InMemoryDataProvider::new_shared();
        let result = lookup_xlookup(&args, &*dummy_provider);
        assert_eq!(result, txt("not found"));
    }

    // --- CHOOSE tests ---

    #[test]
    fn test_choose() {
        let result = lookup_choose(&[num(2.0), txt("a"), txt("b"), txt("c")]);
        assert_eq!(result, txt("b"));
    }

    #[test]
    fn test_choose_out_of_range() {
        let result = lookup_choose(&[num(5.0), txt("a"), txt("b")]);
        assert_eq!(result, err("#VALUE!"));
    }

    // --- ADDRESS tests ---

    #[test]
    fn test_address_absolute() {
        let result = lookup_address(&[num(1.0), num(1.0), num(1.0)]);
        assert_eq!(result, txt("$A$1"));
    }

    #[test]
    fn test_address_relative() {
        // row=2, col=3, abs=4 (relative) -> C2
        let result = lookup_address(&[num(2.0), num(3.0), num(4.0)]);
        assert_eq!(result, txt("C2"));
    }

    #[test]
    fn test_address_with_sheet() {
        let result = lookup_address(&[num(1.0), num(1.0), num(1.0), bool_val(true), txt("Data")]);
        assert_eq!(result, txt("Data!$A$1"));
    }

    // --- ROWS / COLUMNS tests ---

    #[test]
    fn test_rows_with_range() {
        let mut args = Vec::new();
        args.extend(range_marker(3, 5));
        args.extend(vec![CellValue::Empty; 15]); // 3*5
        assert_eq!(lookup_rows(&args), num(5.0));
    }

    #[test]
    fn test_columns_with_range() {
        let mut args = Vec::new();
        args.extend(range_marker(3, 5));
        args.extend(vec![CellValue::Empty; 15]);
        assert_eq!(lookup_columns(&args), num(3.0));
    }

    #[test]
    fn test_rows_flat() {
        // Flat args: treat as 1 row
        assert_eq!(lookup_rows(&[num(1.0), num(2.0)]), num(1.0));
    }

    #[test]
    fn test_columns_flat() {
        // Flat args: treat as N columns
        assert_eq!(lookup_columns(&[num(1.0), num(2.0), num(3.0)]), num(3.0));
    }

    // --- OFFSET / INDIRECT stubs ---

    #[test]
    fn test_offset_no_context() {
        let args = vec![num(1.0), num(2.0), num(3.0)];
        let dummy_provider = InMemoryDataProvider::new_shared();
        assert_eq!(lookup_offset(&args, &*dummy_provider), err("#REF!"));
    }

    #[test]
    fn test_indirect_invalid_ref() {
        let dummy_provider = InMemoryDataProvider::new_shared();
        assert_eq!(
            lookup_indirect(&[txt("not_a_ref")], &*dummy_provider),
            err("#REF!")
        );
    }

    #[test]
    fn test_indirect_no_sheet() {
        // A1 without sheet: no sheet context available
        let dummy_provider = InMemoryDataProvider::new_shared();
        assert_eq!(
            lookup_indirect(&[txt("A1")], &*dummy_provider),
            err("#REF!")
        );
    }

    // --- Value comparison tests ---

    #[test]
    fn test_lookup_values_equal_strings_case_insensitive() {
        assert!(lookup_values_equal(&txt("Hello"), &txt("HELLO")));
        assert!(lookup_values_equal(&txt("hello"), &txt("hello")));
    }

    #[test]
    fn test_lookup_values_equal_numbers() {
        assert!(lookup_values_equal(&num(42.0), &num(42.0)));
        assert!(!lookup_values_equal(&num(42.0), &num(43.0)));
    }

    #[test]
    fn test_lookup_values_equal_cross_type() {
        // Number vs string: never equal in lookup
        assert!(!lookup_values_equal(&num(42.0), &txt("42")));
    }

    // --- A trivial shared provider factory for tests ---

    /// Extend InMemoryDataProvider for tests that need a shared ref.
    use crate::engine::InMemoryDataProvider;

    /// Extension trait to get a shared DataProvider for tests.
    trait InMemoryExt {
        fn new_shared() -> std::sync::Arc<InMemoryDataProvider>;
    }

    impl InMemoryExt for InMemoryDataProvider {
        fn new_shared() -> std::sync::Arc<InMemoryDataProvider> {
            std::sync::Arc::new(InMemoryDataProvider::new())
        }
    }
}
