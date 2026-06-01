use excel_core::cell_ref;
use excel_core::excel_read::read_range;
use excel_types::{CellDiff, DiffType, RangeDiff, Result};

use crate::helpers::classify_diff;

/// Compare a specific range between two files.
pub fn diff_range(
    old_path: &str,
    new_path: &str,
    sheet: &str,
    range_spec: &str,
) -> Result<RangeDiff> {
    let old_data = read_range(old_path, sheet, range_spec)?;
    let new_data = read_range(new_path, sheet, range_spec)?;

    let (r_start, _r_end, c_start, _c_end) = cell_ref::parse_range_normalized(range_spec)?;

    let mut cell_diffs = Vec::new();
    let max_rows = old_data.len().max(new_data.len());
    let max_cols = old_data
        .iter()
        .chain(new_data.iter())
        .map(|r| r.len())
        .max()
        .unwrap_or(0);

    for ri in 0..max_rows {
        for ci in 0..max_cols {
            let abs_row = r_start + ri as u32;
            let abs_col = c_start + ci as u16;
            let old_cell = old_data.get(ri).and_then(|r| r.get(ci));
            let new_cell = new_data.get(ri).and_then(|r| r.get(ci));

            let diff_type = classify_diff(old_cell, new_cell);

            if diff_type != DiffType::NoChange {
                cell_diffs.push(CellDiff {
                    row: abs_row,
                    col: abs_col,
                    cell_ref: cell_ref::format_cell_ref(abs_row, abs_col),
                    diff_type,
                    old_value: old_cell.and_then(|c| c.value.clone()),
                    new_value: new_cell.and_then(|c| c.value.clone()),
                    old_formula: old_cell.and_then(|c| c.formula.clone()),
                    new_formula: new_cell.and_then(|c| c.formula.clone()),
                });
            }
        }
    }

    Ok(RangeDiff {
        range: range_spec.to_string(),
        cell_diffs,
    })
}
