use excel_core::types::{DiffType, Result, SheetDiff};

use crate::engine::compute_cell_diffs;
use crate::helpers::{all_cells_as_diff, read_all_sheets_to_map};

/// Compare a specific sheet between two files.
pub fn diff_sheets(old_path: &str, new_path: &str, sheet: &str) -> Result<SheetDiff> {
    let old_sheets = read_all_sheets_to_map(old_path)?;
    let new_sheets = read_all_sheets_to_map(new_path)?;

    let old_data = old_sheets.get(sheet);
    let new_data = new_sheets.get(sheet);

    let cell_diffs = match (old_data, new_data) {
        (Some(old), Some(new)) => compute_cell_diffs(old, new),
        (Some(old), None) => all_cells_as_diff(old, DiffType::Delete),
        (None, Some(new)) => all_cells_as_diff(new, DiffType::Add),
        (None, None) => Vec::new(),
    };

    let old_rows = old_data.map(|s| s.rows.len() as i32).unwrap_or(0);
    let new_rows = new_data.map(|s| s.rows.len() as i32).unwrap_or(0);
    let old_cols = old_data
        .and_then(|s| s.rows.first().map(|r| r.len() as i32))
        .unwrap_or(0);
    let new_cols = new_data
        .and_then(|s| s.rows.first().map(|r| r.len() as i32))
        .unwrap_or(0);

    Ok(SheetDiff {
        sheet_name: sheet.to_string(),
        row_count_diff: new_rows - old_rows,
        col_count_diff: new_cols - old_cols,
        cell_diffs,
    })
}
