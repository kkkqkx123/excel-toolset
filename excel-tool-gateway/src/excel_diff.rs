use std::collections::HashMap;

use crate::excel_read::read_all_sheets_to_map;
use crate::types::{
    AppError, CellDiff, DiffSummary, DiffType, FileDiff, Result, SheetData, SheetDiff,
};

/// Full file comparison: hash quick-check first, then cell-level diff.
pub fn diff_files(old_path: &str, new_path: &str) -> Result<FileDiff> {
    use crate::security::compute_file_hash;

    let old_hash = compute_file_hash(old_path).map_err(AppError::Io)?;
    let new_hash = compute_file_hash(new_path).map_err(AppError::Io)?;

    if old_hash == new_hash {
        return Ok(FileDiff {
            file_hash_match: true,
            sheet_diffs: Vec::new(),
            summary: DiffSummary {
                adds: 0,
                deletes: 0,
                modifies: 0,
                total_changes: 0,
            },
        });
    }

    let old_sheets = read_all_sheets_to_map(old_path)?;
    let new_sheets = read_all_sheets_to_map(new_path)?;

    let sheet_diffs = diff_sheet_maps(&old_sheets, &new_sheets);
    let summary = summarize(&sheet_diffs);

    Ok(FileDiff {
        file_hash_match: false,
        sheet_diffs,
        summary,
    })
}

/// Compare a specific sheet between two files.
pub fn diff_sheets(
    old_path: &str,
    new_path: &str,
    sheet: &str,
) -> Result<SheetDiff> {
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

/// Compare a specific range between two files.
pub fn diff_range(
    old_path: &str,
    new_path: &str,
    sheet: &str,
    range_spec: &str,
) -> Result<crate::types::RangeDiff> {
    use crate::cell_ref;
    use crate::excel_read::read_range;

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
            let old_cell = old_data.get(ri).and_then(|r| r.get(ci));
            let new_cell = new_data.get(ri).and_then(|r| r.get(ci));

            let diff_type = if old_cell.is_none() && new_cell.is_some() {
                DiffType::Add
            } else if old_cell.is_some() && new_cell.is_none() {
                DiffType::Delete
            } else if let (Some(a), Some(b)) = (old_cell, new_cell) {
                if a.value != b.value || a.formula != b.formula {
                    DiffType::Modify
                } else {
                    DiffType::NoChange
                }
            } else {
                DiffType::NoChange
            };

            if diff_type != DiffType::NoChange {
                cell_diffs.push(CellDiff {
                    row: r_start + ri as u32,
                    col: c_start + ci as u16,
                    diff_type,
                    old_value: old_cell.and_then(|c| c.value.clone()),
                    new_value: new_cell.and_then(|c| c.value.clone()),
                    old_formula: old_cell.and_then(|c| c.formula.clone()),
                    new_formula: new_cell.and_then(|c| c.formula.clone()),
                });
            }
        }
    }

    Ok(crate::types::RangeDiff {
        range: range_spec.to_string(),
        cell_diffs,
    })
}

/// Compute diff between two in-memory sheet data maps (for write operations).
pub(crate) fn diff_sheet_maps(
    old: &HashMap<String, SheetData>,
    new: &HashMap<String, SheetData>,
) -> Vec<SheetDiff> {
    let mut all_sheets: Vec<String> = old.keys().cloned().collect();
    for k in new.keys() {
        if !all_sheets.contains(k) {
            all_sheets.push(k.clone());
        }
    }
    all_sheets.sort();

    let mut diffs = Vec::new();
    for sheet_name in all_sheets {
        let old_sheet = old.get(&sheet_name);
        let new_sheet = new.get(&sheet_name);

        let cell_diffs = match (old_sheet, new_sheet) {
            (Some(a), Some(b)) => compute_cell_diffs(a, b),
            (Some(a), None) => all_cells_as_diff(a, DiffType::Delete),
            (None, Some(b)) => all_cells_as_diff(b, DiffType::Add),
            (None, None) => Vec::new(),
        };

        let old_rows = old_sheet.map(|s| s.rows.len() as i32).unwrap_or(0);
        let new_rows = new_sheet.map(|s| s.rows.len() as i32).unwrap_or(0);
        let old_cols = old_sheet
            .and_then(|s| s.rows.first().map(|r| r.len() as i32))
            .unwrap_or(0);
        let new_cols = new_sheet
            .and_then(|s| s.rows.first().map(|r| r.len() as i32))
            .unwrap_or(0);

        diffs.push(SheetDiff {
            sheet_name,
            row_count_diff: new_rows - old_rows,
            col_count_diff: new_cols - old_cols,
            cell_diffs,
        });
    }

    diffs
}

/// Compute cell-level diff between two SheetData structs.
pub(crate) fn compute_cell_diffs(old: &SheetData, new: &SheetData) -> Vec<CellDiff> {
    let mut diffs = Vec::new();
    let max_rows = old.rows.len().max(new.rows.len());

    for ri in 0..max_rows {
        let old_row = old.rows.get(ri);
        let new_row = new.rows.get(ri);
        let max_cols = old_row
            .map(|r| r.len())
            .unwrap_or(0)
            .max(new_row.map(|r| r.len()).unwrap_or(0));

        for ci in 0..max_cols {
            let old_cell = old_row.and_then(|r| r.get(ci));
            let new_cell = new_row.and_then(|r| r.get(ci));

            let diff_type = match (&old_cell, &new_cell) {
                (None, Some(_)) => DiffType::Add,
                (Some(_), None) => DiffType::Delete,
                (Some(a), Some(b)) => {
                    if a.value != b.value || a.formula != b.formula {
                        DiffType::Modify
                    } else {
                        DiffType::NoChange
                    }
                }
                (None, None) => DiffType::NoChange,
            };

            if diff_type != DiffType::NoChange {
                diffs.push(CellDiff {
                    row: ri as u32,
                    col: ci as u16,
                    diff_type,
                    old_value: old_cell.and_then(|c| c.value.clone()),
                    new_value: new_cell.and_then(|c| c.value.clone()),
                    old_formula: old_cell.and_then(|c| c.formula.clone()),
                    new_formula: new_cell.and_then(|c| c.formula.clone()),
                });
            }
        }
    }

    diffs
}

fn all_cells_as_diff(data: &SheetData, diff_type: DiffType) -> Vec<CellDiff> {
    let mut diffs = Vec::new();
    for (ri, row) in data.rows.iter().enumerate() {
        for (ci, cell) in row.iter().enumerate() {
            diffs.push(CellDiff {
                row: ri as u32,
                col: ci as u16,
                diff_type: diff_type.clone(),
                old_value: if diff_type == DiffType::Delete {
                    cell.value.clone()
                } else {
                    None
                },
                new_value: if diff_type == DiffType::Add {
                    cell.value.clone()
                } else {
                    None
                },
                old_formula: if diff_type == DiffType::Delete {
                    cell.formula.clone()
                } else {
                    None
                },
                new_formula: if diff_type == DiffType::Add {
                    cell.formula.clone()
                } else {
                    None
                },
            });
        }
    }
    diffs
}

fn summarize(sheet_diffs: &[SheetDiff]) -> DiffSummary {
    let mut adds = 0;
    let mut deletes = 0;
    let mut modifies = 0;

    for sd in sheet_diffs {
        for cd in &sd.cell_diffs {
            match cd.diff_type {
                DiffType::Add => adds += 1,
                DiffType::Delete => deletes += 1,
                DiffType::Modify => modifies += 1,
                DiffType::NoChange => {}
            }
        }
    }

    DiffSummary {
        adds,
        deletes,
        modifies,
        total_changes: adds + deletes + modifies,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CellData, CellDataType};

    #[test]
    fn test_diff_identical_returns_no_changes() {
        let mut old = HashMap::new();
        old.insert(
            "Sheet1".to_string(),
            SheetData {
                name: "Sheet1".to_string(),
                rows: vec![vec![
                    CellData {
                        value: Some("hello".into()),
                        data_type: CellDataType::String,
                        formula: None,
                    },
                ]],
            },
        );
        let new = old.clone();

        let diffs = diff_sheet_maps(&old, &new);
        assert!(diffs.is_empty() || diffs[0].cell_diffs.is_empty());
    }

    #[test]
    fn test_diff_detects_modification() {
        let old = sheet_data_with_values(&["hello"]);
        let new = sheet_data_with_values(&["world"]);

        let diffs = compute_cell_diffs(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Modify);
        assert_eq!(diffs[0].old_value, Some("hello".into()));
        assert_eq!(diffs[0].new_value, Some("world".into()));
    }

    fn sheet_data_with_values(values: &[&str]) -> SheetData {
        SheetData {
            name: "Sheet1".to_string(),
            rows: vec![values
                .iter()
                .map(|v| CellData {
                    value: Some(v.to_string()),
                    data_type: CellDataType::String,
                    formula: None,
                })
                .collect()],
        }
    }
}
