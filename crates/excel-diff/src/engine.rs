use std::collections::HashMap;

use excel_types::{CellDiff, DiffType, SheetData, SheetDiff};

pub fn compute_cell_diffs(old_data: &SheetData, new_data: &SheetData) -> Vec<CellDiff> {
    let mut diffs = Vec::new();

    let max_row = std::cmp::max(old_data.rows.len(), new_data.rows.len());

    for row_idx in 0..max_row {
        let old_row = old_data.rows.get(row_idx);
        let new_row = new_data.rows.get(row_idx);

        let max_col = match (old_row, new_row) {
            (Some(or), Some(nr)) => std::cmp::max(or.len(), nr.len()),
            (Some(or), None) => or.len(),
            (None, Some(nr)) => nr.len(),
            (None, None) => continue,
        };

        for col_idx in 0..max_col {
            let old_cell = old_row.and_then(|r| r.get(col_idx));
            let new_cell = new_row.and_then(|r| r.get(col_idx));

            if old_cell.is_none() && new_cell.is_none() {
                continue;
            }

            let diff_type = crate::helpers::classify_diff(old_cell, new_cell);

            if diff_type != DiffType::NoChange {
                let cell_ref = format_cell_ref(row_idx, col_idx);
                diffs.push(CellDiff {
                    row: row_idx as u32,
                    col: col_idx as u16,
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

fn format_cell_ref(row: usize, col: usize) -> String {
    let col_name = index_to_col(col);
    let row_num = row + 1;
    format!("{}{}", col_name, row_num)
}

fn index_to_col(index: usize) -> String {
    let mut col = String::new();
    let mut n = index + 1;

    while n > 0 {
        n -= 1;
        col.insert(0, ((n % 26) as u8 + b'A') as char);
        n /= 26;
    }

    col
}

#[cfg(test)]
mod tests {
    use super::*;
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
