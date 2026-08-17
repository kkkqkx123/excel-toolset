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

use crate::engine::FunctionImpl;
use crate::evaluator::partial_cmp_cell_values;

// ---------------------------------------------------------------------------
// Registry entry point
// ---------------------------------------------------------------------------
pub fn register(registry: &mut HashMap<String, FunctionImpl>) {
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
pub(crate) const RANGE_MARKER_THRESHOLD: f64 = -900_000.0;

pub(crate) const RANGE_MARKER_OFFSET: f64 = 1_000_000.0;

/// Check whether a CellValue is a range dimension sentinel.
#[allow(dead_code)]
pub(crate) fn is_range_marker(val: &CellValue) -> bool {
    matches!(val, CellValue::Number(n) if *n < RANGE_MARKER_THRESHOLD)
}

/// Decode the column count from a sentinel marker value.
pub(crate) fn decode_cols(marker: f64) -> usize {
    ((-marker) - RANGE_MARKER_OFFSET) as usize
}

/// Try to consume a range marker starting at `args[start]`.
///
/// On success returns `Some((cols, rows, data_end))` where `data_end` is the
/// index of the first arg *after* the expanded range values.
pub(crate) fn consume_range_marker(
    args: &[CellValue],
    start: usize,
) -> Option<(usize, usize, usize)> {
    if start >= args.len() {
        return None;
    }
    if let CellValue::Number(n) = &args[start]
        && *n < RANGE_MARKER_THRESHOLD
    {
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
    None
}

/// Build a 2D table ```table[row][col]``` from flat range data.
pub(crate) fn build_2d_table(data: &[CellValue], cols: usize, rows: usize) -> Vec<Vec<CellValue>> {
    let mut table = Vec::with_capacity(rows);
    for r in 0..rows {
        let start = r * cols;
        table.push(data[start..start + cols].to_vec());
    }
    table
}

/// Extract the first column from a 2D table.
pub(crate) fn extract_column(table: &[Vec<CellValue>], col_idx: usize) -> Vec<&CellValue> {
    table.iter().map(|row| &row[col_idx]).collect()
}

// ---------------------------------------------------------------------------
// Exact / approximate match utilities
// ---------------------------------------------------------------------------

/// Excel-style equality for lookup exact match.
///
/// Strings are compared case-insensitively. Cross-type comparisons
/// (e.g., number vs string) always return false.
pub(crate) fn lookup_values_equal(a: &CellValue, b: &CellValue) -> bool {
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
pub(crate) fn exact_match(lookup_value: &CellValue, haystack: &[&CellValue]) -> Option<usize> {
    haystack
        .iter()
        .position(|v| lookup_values_equal(v, lookup_value))
}

/// Direction for approximate match search.
pub(crate) enum ApproxMode {
    /// Find the largest value <= lookup (ascending data).
    LargestLe,
    /// Find the smallest value >= lookup (descending data).
    SmallestGe,
}

/// Perform approximate match search in a sorted slice of CellValues.
///
/// `mode` controls the direction: `LargestLe` for VLOOKUP/HLOOKUP/type-1 MATCH,
/// `SmallestGe` for type-(-1) MATCH.
pub(crate) fn approximate_match(
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
pub(crate) fn col_letters_to_index(s: &str) -> Option<u32> {
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
pub(crate) fn parse_a1_ref(text: &str) -> Option<(String, u32, u32)> {
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

/// Convert a 0-based column index to Excel column letters (A, B, ..., Z, AA, ...).
pub(crate) fn col_index_to_letters(mut col: u32) -> String {
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

// ── 各查找/引用函数实现（每函数一个文件）──
mod choose_address;
mod hlookup;
mod index_match;
mod offset_indirect;
mod row_column;
mod vlookup;
mod xlookup;

use self::{
    choose_address::lookup_address, choose_address::lookup_choose, hlookup::lookup_hlookup,
    index_match::lookup_index, index_match::lookup_match, offset_indirect::lookup_indirect,
    offset_indirect::lookup_offset, row_column::lookup_column, row_column::lookup_columns,
    row_column::lookup_row, row_column::lookup_rows, vlookup::lookup_vlookup,
    xlookup::lookup_xlookup,
};

#[cfg(test)]
mod tests;
