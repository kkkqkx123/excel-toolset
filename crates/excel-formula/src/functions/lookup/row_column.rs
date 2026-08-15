use excel_types::CellValue;

use super::*;

pub(crate) fn lookup_row(_args: &[CellValue]) -> CellValue {
    // Without cell coordinate context, return 1.
    CellValue::Number(1.0)
}

// ---------------------------------------------------------------------------
// COLUMN([reference])
// ---------------------------------------------------------------------------


pub(crate) fn lookup_column(_args: &[CellValue]) -> CellValue {
    // Without cell coordinate context, return 1.
    CellValue::Number(1.0)
}

// ---------------------------------------------------------------------------
// ROWS(array)
// ---------------------------------------------------------------------------


pub(crate) fn lookup_rows(args: &[CellValue]) -> CellValue {
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


pub(crate) fn lookup_columns(args: &[CellValue]) -> CellValue {
    if let Some((cols, _rows, _end)) = consume_range_marker(args, 0) {
        return CellValue::Number(cols as f64);
    }
    // No range marker: treat flat args as a single row, count columns = len(args).
    CellValue::Number(args.len() as f64)
}

// ---------------------------------------------------------------------------
// CHOOSE(index_num, value1, [value2], ...)
// ---------------------------------------------------------------------------

