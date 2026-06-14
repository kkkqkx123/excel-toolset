use std::collections::HashMap;

use excel_core::excel_read;
use excel_types::{CellData, CellDiff, DiffType, Result, SheetData};

pub fn classify_diff(old_cell: Option<&CellData>, new_cell: Option<&CellData>) -> DiffType {
    match (old_cell, new_cell) {
        (None, None) => DiffType::NoChange,
        (None, Some(_)) => DiffType::Add,
        (Some(_), None) => DiffType::Delete,
        (Some(old), Some(new)) => {
            if old.formula.is_some() && new.formula.is_some() {
                if old.formula == new.formula {
                    if old.value != new.value {
                        return DiffType::Passive;
                    }
                    return DiffType::NoChange;
                }
                return DiffType::Modify;
            }

            if old.value == new.value {
                return DiffType::NoChange;
            }

            DiffType::Modify
        }
    }
}

pub fn all_cells_as_diff(sheet: &SheetData, diff_type: DiffType) -> Vec<CellDiff> {
    let mut diffs = Vec::new();

    for (row_idx, row) in sheet.rows.iter().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            let cell_ref = format_cell_ref(row_idx, col_idx);

            diffs.push(CellDiff {
                row: row_idx as u32,
                col: col_idx as u16,
                cell_ref,
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

pub fn read_all_sheets_to_map(path: &str) -> Result<HashMap<String, SheetData>> {
    let sheet_names = excel_read::list_sheets(path)?;
    let mut sheets = HashMap::new();

    for sheet_name in &sheet_names {
        let sheet = excel_read::read_sheet_all(path, sheet_name)?;
        sheets.insert(sheet_name.clone(), sheet);
    }

    Ok(sheets)
}

pub fn format_cell_ref(row: usize, col: usize) -> String {
    let col_name = index_to_col(col);
    let row_num = row + 1;
    format!("{}{}", col_name, row_num)
}

pub fn index_to_col(index: usize) -> String {
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
    use excel_types::CellDataType;

    fn cell(value: &str) -> CellData {
        CellData {
            value: Some(value.into()),
            data_type: CellDataType::String,
            formula: None,
        }
    }

    fn formula_cell(value: &str, formula: &str) -> CellData {
        CellData {
            value: Some(value.into()),
            data_type: CellDataType::String,
            formula: Some(formula.into()),
        }
    }

    #[test]
    fn test_classify_both_none() {
        assert_eq!(classify_diff(None, None), DiffType::NoChange);
    }

    #[test]
    fn test_classify_add() {
        assert_eq!(classify_diff(None, Some(&cell("A"))), DiffType::Add);
    }

    #[test]
    fn test_classify_delete() {
        assert_eq!(classify_diff(Some(&cell("A")), None), DiffType::Delete);
    }

    #[test]
    fn test_classify_modify_value() {
        assert_eq!(
            classify_diff(Some(&cell("A")), Some(&cell("B"))),
            DiffType::Modify
        );
    }

    #[test]
    fn test_classify_no_change() {
        assert_eq!(
            classify_diff(Some(&cell("A")), Some(&cell("A"))),
            DiffType::NoChange
        );
    }

    #[test]
    fn test_classify_passive_formula_same_value_changed() {
        let old = formula_cell("10", "=A1+1");
        let new = formula_cell("11", "=A1+1");
        assert_eq!(classify_diff(Some(&old), Some(&new)), DiffType::Passive);
    }

    #[test]
    fn test_classify_modify_formula_changed() {
        let old = formula_cell("10", "=A1+1");
        let new = formula_cell("20", "=A1+2");
        assert_eq!(classify_diff(Some(&old), Some(&new)), DiffType::Modify);
    }

    #[test]
    fn test_classify_modify_formula_changed_same_value() {
        let old = formula_cell("10", "=A1+1");
        let new = formula_cell("10", "=B1+1");
        assert_eq!(classify_diff(Some(&old), Some(&new)), DiffType::Modify);
    }

    #[test]
    fn test_all_cells_as_diff_add() {
        let data = SheetData {
            name: "S".into(),
            rows: vec![vec![cell("A1"), cell("B1")], vec![cell("A2")]],
        };
        let diffs = all_cells_as_diff(&data, DiffType::Add);
        assert_eq!(diffs.len(), 3);
        for d in &diffs {
            assert_eq!(d.diff_type, DiffType::Add);
            assert!(d.new_value.is_some());
            assert!(d.old_value.is_none());
            assert!(d.old_formula.is_none());
            assert!(d.new_formula.is_none());
        }
        assert_eq!(diffs[0].cell_ref, "A1");
        assert_eq!(diffs[1].cell_ref, "B1");
        assert_eq!(diffs[2].cell_ref, "A2");
    }

    #[test]
    fn test_all_cells_as_diff_delete() {
        let data = SheetData {
            name: "S".into(),
            rows: vec![vec![cell("X")]],
        };
        let diffs = all_cells_as_diff(&data, DiffType::Delete);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Delete);
        assert!(diffs[0].old_value.is_some());
        assert!(diffs[0].new_value.is_none());
    }

    #[test]
    fn test_all_cells_as_diff_empty_sheet() {
        let data = SheetData {
            name: "S".into(),
            rows: vec![],
        };
        let diffs = all_cells_as_diff(&data, DiffType::Add);
        assert!(diffs.is_empty());
    }

    #[test]
    fn test_read_all_sheets_to_map_returns_error_on_bad_path() {
        let result = read_all_sheets_to_map("nonexistent_file.xlsx");
        assert!(result.is_err());
    }

    #[test]
    fn test_classify_old_formula_new_no_formula_same_value() {
        let old = formula_cell("10", "=A1+1");
        let new = cell("10");
        // Currently: same value → NoChange (formula removal not considered a diff alone)
        assert_eq!(classify_diff(Some(&old), Some(&new)), DiffType::NoChange);
    }

    #[test]
    fn test_classify_old_formula_new_no_formula_diff_value() {
        let old = formula_cell("10", "=A1+1");
        let new = cell("20");
        // Diff value with mixed formula presence → Modify
        assert_eq!(classify_diff(Some(&old), Some(&new)), DiffType::Modify);
    }

    #[test]
    fn test_classify_old_no_formula_new_formula() {
        let old = cell("10");
        let new = formula_cell("10", "=A1+1");
        // Currently: same value → NoChange
        assert_eq!(classify_diff(Some(&old), Some(&new)), DiffType::NoChange);
    }

    #[test]
    fn test_classify_formula_same_value_same_formula() {
        let old = formula_cell("10", "=A1+1");
        let new = formula_cell("10", "=A1+1");
        assert_eq!(classify_diff(Some(&old), Some(&new)), DiffType::NoChange);
    }

    #[test]
    fn test_all_cells_as_diff_add_with_formulas() {
        let data = SheetData {
            name: "S".into(),
            rows: vec![vec![formula_cell("10", "=A2+1")]],
        };
        let diffs = all_cells_as_diff(&data, DiffType::Add);
        assert_eq!(diffs.len(), 1);
        assert!(diffs[0].new_formula.is_some());
        assert_eq!(diffs[0].new_formula.as_deref(), Some("=A2+1"));
        assert!(diffs[0].old_formula.is_none());
    }

    #[test]
    fn test_all_cells_as_diff_delete_with_formulas() {
        let data = SheetData {
            name: "S".into(),
            rows: vec![vec![formula_cell("10", "=A2+1")]],
        };
        let diffs = all_cells_as_diff(&data, DiffType::Delete);
        assert_eq!(diffs.len(), 1);
        assert!(diffs[0].old_formula.is_some());
        assert_eq!(diffs[0].old_formula.as_deref(), Some("=A2+1"));
        assert!(diffs[0].new_formula.is_none());
    }

    #[test]
    fn test_classify_both_none_skip() {
        assert_eq!(
            classify_diff(Some(&cell("A")), Some(&cell("A"))),
            DiffType::NoChange
        );
    }
}
