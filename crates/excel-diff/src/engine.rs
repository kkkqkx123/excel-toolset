use std::collections::HashMap;

use excel_core::cell_ref;
use excel_core::types::{CellDiff, DiffType, SheetData, SheetDiff};

use crate::helpers::all_cells_as_diff;

/// Compute cell-level diff between two SheetData structs.
pub fn compute_cell_diffs(old: &SheetData, new: &SheetData) -> Vec<CellDiff> {
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
            let cell_ref_str = cell_ref::format_cell_ref(ri as u32, ci as u16);

            let diff_type = crate::helpers::classify_diff(old_cell, new_cell);

            if diff_type != DiffType::NoChange {
                diffs.push(CellDiff {
                    row: ri as u32,
                    col: ci as u16,
                    cell_ref: cell_ref_str,
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

/// Compute diff between two in-memory sheet data maps.
pub fn diff_sheet_maps(
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

#[cfg(test)]
mod tests {
    use super::*;
    use excel_core::types::{CellData, CellDataType};
    use std::collections::HashMap;

    #[test]
    fn test_diff_identical_returns_no_changes() {
        let mut old = HashMap::new();
        old.insert(
            "Sheet1".to_string(),
            SheetData {
                name: "Sheet1".to_string(),
                rows: vec![vec![CellData {
                    value: Some("hello".into()),
                    data_type: CellDataType::String,
                    formula: None,
                }]],
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

    #[test]
    fn test_diff_empty_to_data() {
        let old = SheetData {
            name: "S".into(),
            rows: vec![],
        };
        let new = SheetData {
            name: "S".into(),
            rows: vec![vec![CellData {
                value: Some("added".into()),
                data_type: CellDataType::String,
                formula: None,
            }]],
        };
        let diffs = compute_cell_diffs(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Add);
        assert_eq!(diffs[0].cell_ref, "A1");
    }

    #[test]
    fn test_diff_data_to_empty() {
        let old = SheetData {
            name: "S".into(),
            rows: vec![vec![CellData {
                value: Some("removed".into()),
                data_type: CellDataType::String,
                formula: None,
            }]],
        };
        let new = SheetData {
            name: "S".into(),
            rows: vec![],
        };
        let diffs = compute_cell_diffs(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Delete);
    }

    #[test]
    fn test_diff_cell_ref_formatting() {
        let new = SheetData {
            name: "S".into(),
            rows: vec![
                vec![CellData {
                    value: Some("A1".into()),
                    data_type: CellDataType::String,
                    formula: None,
                }],
                vec![CellData {
                    value: Some("B1".into()),
                    data_type: CellDataType::String,
                    formula: None,
                }],
            ],
        };
        let diffs = compute_cell_diffs(
            &SheetData {
                name: "S".into(),
                rows: vec![],
            },
            &new,
        );
        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[0].cell_ref, "A1");
        assert_eq!(diffs[1].cell_ref, "A2");
    }

    #[test]
    fn test_diff_sheet_maps_add_sheet() {
        let mut old = HashMap::new();
        old.insert("S1".into(), empty_sheet());

        let mut new = HashMap::new();
        new.insert("S1".into(), empty_sheet());
        new.insert(
            "S2".into(),
            SheetData {
                name: "S2".into(),
                rows: vec![vec![CellData {
                    value: Some("x".into()),
                    data_type: CellDataType::String,
                    formula: None,
                }]],
            },
        );

        let diffs = diff_sheet_maps(&old, &new);
        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[1].sheet_name, "S2");
        assert!(!diffs[1].cell_diffs.is_empty());
    }

    #[test]
    fn test_diff_sheet_maps_delete_sheet() {
        let mut old = HashMap::new();
        old.insert("S1".into(), empty_sheet());
        old.insert("S2".into(), empty_sheet());

        let mut new = HashMap::new();
        new.insert("S1".into(), empty_sheet());

        let diffs = diff_sheet_maps(&old, &new);
        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[1].sheet_name, "S2");
    }

    #[test]
    fn test_diff_sheet_maps_row_count_diff() {
        let mut old = HashMap::new();
        old.insert(
            "S1".into(),
            SheetData {
                name: "S1".into(),
                rows: vec![vec![]],
            },
        );

        let mut new = HashMap::new();
        new.insert(
            "S1".into(),
            SheetData {
                name: "S1".into(),
                rows: vec![vec![], vec![]],
            },
        );

        let diffs = diff_sheet_maps(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].row_count_diff, 1);
    }

    fn empty_sheet() -> SheetData {
        SheetData {
            name: "S".into(),
            rows: vec![],
        }
    }

    fn sheet_data_with_values(values: &[&str]) -> SheetData {
        SheetData {
            name: "Sheet1".to_string(),
            rows: vec![
                values
                    .iter()
                    .map(|v| CellData {
                        value: Some(v.to_string()),
                        data_type: CellDataType::String,
                        formula: None,
                    })
                    .collect(),
            ],
        }
    }
}
