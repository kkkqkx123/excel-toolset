use std::collections::HashMap;

use crate::helpers::format_cell_ref;
use excel_types::{CellDiff, DiffType, SheetData, SheetDiff};

/// Compare two sheets cell-by-cell in **absolute** worksheet coordinates.
///
/// `SheetData::rows` is stored relative to the sheet's used-range origin
/// (`start_row`/`start_col`). Comparing raw relative indices would both
/// mis-label the reported cell reference (a change in `E3` reported as `C1`)
/// and mis-align two sheets whose used-ranges start at different origins.
/// Everything below therefore works in absolute indices and converts back to
/// per-sheet relative indices only when fetching a cell.
pub fn compute_cell_diffs(old_data: &SheetData, new_data: &SheetData) -> Vec<CellDiff> {
    let mut diffs = Vec::new();

    let old_r0 = old_data.start_row as usize;
    let old_c0 = old_data.start_col as usize;
    let new_r0 = new_data.start_row as usize;
    let new_c0 = new_data.start_col as usize;

    let first_row = old_r0.min(new_r0);
    let last_row = (old_r0 + old_data.rows.len()).max(new_r0 + new_data.rows.len());

    for abs_row in first_row..last_row {
        let old_row = abs_row
            .checked_sub(old_r0)
            .and_then(|i| old_data.rows.get(i));
        let new_row = abs_row
            .checked_sub(new_r0)
            .and_then(|i| new_data.rows.get(i));

        let first_col = old_c0.min(new_c0);
        let last_col = match (old_row, new_row) {
            (Some(or), Some(nr)) => (old_c0 + or.len()).max(new_c0 + nr.len()),
            (Some(or), None) => old_c0 + or.len(),
            (None, Some(nr)) => new_c0 + nr.len(),
            (None, None) => continue,
        };

        for abs_col in first_col..last_col {
            let old_cell =
                old_row.and_then(|r| abs_col.checked_sub(old_c0).and_then(|i| r.get(i)));
            let new_cell =
                new_row.and_then(|r| abs_col.checked_sub(new_c0).and_then(|i| r.get(i)));

            if old_cell.is_none() && new_cell.is_none() {
                continue;
            }

            let diff_type = crate::helpers::classify_diff(old_cell, new_cell);

            if diff_type != DiffType::NoChange {
                let cell_ref = format_cell_ref(abs_row, abs_col);
                diffs.push(CellDiff {
                    row: abs_row as u32,
                    col: abs_col as u16,
                    cell_ref,
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

pub fn diff_sheet_maps(
    old: &HashMap<String, SheetData>,
    new: &HashMap<String, SheetData>,
) -> Vec<SheetDiff> {
    let mut diffs = Vec::new();

    let all_sheet_names: std::collections::BTreeSet<_> = old.keys().chain(new.keys()).collect();

    for sheet_name in all_sheet_names {
        let old_sheet = old.get(sheet_name);
        let new_sheet = new.get(sheet_name);

        let row_count_diff = match (old_sheet, new_sheet) {
            (Some(os), Some(ns)) => ns.rows.len() as i32 - os.rows.len() as i32,
            (Some(_), None) => -(old_sheet.unwrap().rows.len() as i32),
            (None, Some(_)) => new_sheet.unwrap().rows.len() as i32,
            (None, None) => 0,
        };

        let col_count_diff = match (old_sheet, new_sheet) {
            (Some(os), Some(ns)) => {
                let old_max = os.rows.iter().map(|r| r.len()).max().unwrap_or(0);
                let new_max = ns.rows.iter().map(|r| r.len()).max().unwrap_or(0);
                new_max as i32 - old_max as i32
            }
            (Some(os), None) => {
                let max = os.rows.iter().map(|r| r.len()).max().unwrap_or(0);
                -(max as i32)
            }
            (None, Some(ns)) => {
                let max = ns.rows.iter().map(|r| r.len()).max().unwrap_or(0);
                max as i32
            }
            (None, None) => 0,
        };

        let cell_diffs = match (old_sheet, new_sheet) {
            (Some(os), Some(ns)) => compute_cell_diffs(os, ns),
            (Some(os), None) => crate::helpers::all_cells_as_diff(os, DiffType::Delete),
            (None, Some(ns)) => crate::helpers::all_cells_as_diff(ns, DiffType::Add),
            (None, None) => Vec::new(),
        };

        diffs.push(SheetDiff {
            sheet_name: sheet_name.clone(),
            cell_diffs,
            row_count_diff,
            col_count_diff,
        });
    }

    diffs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::index_to_col;
    use excel_types::{CellData, CellDataType};
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
                ..Default::default()
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
            ..Default::default()
        };
        let new = SheetData {
            name: "S".into(),
            rows: vec![vec![CellData {
                value: Some("added".into()),
                data_type: CellDataType::String,
                formula: None,
            }]],
            ..Default::default()
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
            ..Default::default()
        };
        let new = SheetData {
            name: "S".into(),
            rows: vec![],
            ..Default::default()
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
            ..Default::default()
        };
        let diffs = compute_cell_diffs(
            &SheetData {
                name: "S".into(),
                rows: vec![],
                ..Default::default()
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
                ..Default::default()
            },
        );

        let mut diffs = diff_sheet_maps(&old, &new);
        diffs.sort_by(|a, b| a.sheet_name.cmp(&b.sheet_name));
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

        let mut diffs = diff_sheet_maps(&old, &new);
        diffs.sort_by(|a, b| a.sheet_name.cmp(&b.sheet_name));
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
                ..Default::default()
            },
        );

        let mut new = HashMap::new();
        new.insert(
            "S1".into(),
            SheetData {
                name: "S1".into(),
                rows: vec![vec![], vec![]],
                ..Default::default()
            },
        );

        let diffs = diff_sheet_maps(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].row_count_diff, 1);
    }

    #[test]
    fn test_diff_sheet_maps_add_sheet_col_count() {
        let old = HashMap::new();

        let mut new = HashMap::new();
        new.insert(
            "S1".into(),
            SheetData {
                name: "S1".into(),
                rows: vec![
                    vec![CellData {
                        value: Some("h1".into()),
                        data_type: CellDataType::String,
                        formula: None,
                    }],
                    vec![
                        CellData {
                            value: Some("h2".into()),
                            data_type: CellDataType::String,
                            formula: None,
                        },
                        CellData {
                            value: Some("h3".into()),
                            data_type: CellDataType::String,
                            formula: None,
                        },
                    ],
                ],
                ..Default::default()
            },
        );

        let diffs = diff_sheet_maps(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].col_count_diff, 2);
        assert_eq!(diffs[0].row_count_diff, 2);
    }

    #[test]
    fn test_diff_sheet_maps_delete_sheet_col_count() {
        let mut old = HashMap::new();
        old.insert(
            "S1".into(),
            SheetData {
                name: "S1".into(),
                rows: vec![vec![CellData {
                    value: Some("x".into()),
                    data_type: CellDataType::String,
                    formula: None,
                }]],
                ..Default::default()
            },
        );

        let new = HashMap::new();

        let diffs = diff_sheet_maps(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].col_count_diff, -1);
        assert_eq!(diffs[0].row_count_diff, -1);
    }

    #[test]
    fn test_index_to_col_single_letter() {
        assert_eq!(index_to_col(0), "A");
        assert_eq!(index_to_col(25), "Z");
    }

    #[test]
    fn test_index_to_col_multi_letter() {
        assert_eq!(index_to_col(26), "AA");
        assert_eq!(index_to_col(701), "ZZ");
        assert_eq!(index_to_col(702), "AAA");
    }

    #[test]
    fn test_format_cell_ref() {
        assert_eq!(format_cell_ref(0, 0), "A1");
        assert_eq!(format_cell_ref(0, 26), "AA1");
        assert_eq!(format_cell_ref(99, 0), "A100");
    }

    #[test]
    fn test_diff_sheet_maps_same_sheets_both_exist_no_changes() {
        let old_data = sheet_data_with_values(&["a"]);
        let new_data = sheet_data_with_values(&["a"]);

        let mut old = HashMap::new();
        old.insert("S1".into(), old_data);
        let mut new = HashMap::new();
        new.insert("S1".into(), new_data);

        let diffs = diff_sheet_maps(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert!(diffs[0].cell_diffs.is_empty());
        assert_eq!(diffs[0].row_count_diff, 0);
        assert_eq!(diffs[0].col_count_diff, 0);
    }

    fn empty_sheet() -> SheetData {
        SheetData {
            name: "S".into(),
            rows: vec![],
            ..Default::default()
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
            ..Default::default()
        }
    }
}
