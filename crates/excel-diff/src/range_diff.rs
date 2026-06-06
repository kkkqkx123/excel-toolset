use excel_core::excel_read::read_range;
use excel_core::utils::cell_ref;
use excel_types::{CellData, CellDiff, DiffType, RangeDiff, Result};

use crate::helpers::classify_diff;

/// Pure core: compare two cell-data matrices and produce diffs with absolute coordinates.
pub fn compute_range_cell_diffs(
    old_data: &[Vec<CellData>],
    new_data: &[Vec<CellData>],
    r_start: u32,
    c_start: u16,
) -> Vec<CellDiff> {
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

    cell_diffs
}

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

    let cell_diffs = compute_range_cell_diffs(&old_data, &new_data, r_start, c_start);

    Ok(RangeDiff {
        range: range_spec.to_string(),
        cell_diffs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use excel_types::CellDataType;

    fn cell_data(value: &str) -> CellData {
        CellData {
            value: Some(value.into()),
            data_type: CellDataType::String,
            formula: None,
        }
    }

    #[test]
    fn test_compute_range_identical() {
        let data = vec![vec![cell_data("a"), cell_data("b")]];
        let diffs = compute_range_cell_diffs(&data, &data, 0, 0);
        assert!(diffs.is_empty());
    }

    #[test]
    fn test_compute_range_modify() {
        let old = vec![vec![cell_data("old")]];
        let new = vec![vec![cell_data("new")]];
        let diffs = compute_range_cell_diffs(&old, &new, 0, 0);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].cell_ref, "A1");
        assert_eq!(diffs[0].diff_type, DiffType::Modify);
    }

    #[test]
    fn test_compute_range_absolute_coordinates() {
        let old = vec![vec![cell_data("x")]];
        let new = vec![vec![cell_data("y")]];
        let diffs = compute_range_cell_diffs(&old, &new, 4, 2);
        assert_eq!(diffs[0].row, 4);
        assert_eq!(diffs[0].col, 2);
        assert_eq!(diffs[0].cell_ref, "C5");
    }

    #[test]
    fn test_compute_range_add_row() {
        let old: Vec<Vec<CellData>> = vec![];
        let new = vec![vec![cell_data("added")]];
        let diffs = compute_range_cell_diffs(&old, &new, 0, 0);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Add);
    }

    #[test]
    fn test_compute_range_mismatched_cols() {
        let old = vec![vec![cell_data("a"), cell_data("b")]];
        let new = vec![vec![cell_data("a")]];
        let diffs = compute_range_cell_diffs(&old, &new, 0, 0);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Delete);
        assert_eq!(diffs[0].cell_ref, "B1");
    }

    #[test]
    fn test_compute_range_empty_both() {
        let diffs = compute_range_cell_diffs(&[], &[], 0, 0);
        assert!(diffs.is_empty());
    }
}
