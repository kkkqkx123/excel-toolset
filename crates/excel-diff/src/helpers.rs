use std::collections::HashMap;

use excel_core::cell_ref;
use excel_core::excel_read;
use excel_core::types::{CellData, CellDiff, DiffType, Result, SheetData};

pub(crate) fn read_all_sheets_to_map(path: &str) -> Result<HashMap<String, SheetData>> {
    let sheets = excel_read::list_sheets(path)?;
    let mut map = HashMap::new();
    for name in sheets {
        let data = excel_read::read_sheet_all(path, &name)?;
        map.insert(name, data);
    }
    Ok(map)
}

pub(crate) fn all_cells_as_diff(data: &SheetData, diff_type: DiffType) -> Vec<CellDiff> {
    let mut diffs = Vec::new();
    for (ri, row) in data.rows.iter().enumerate() {
        for (ci, cell) in row.iter().enumerate() {
            diffs.push(CellDiff {
                row: ri as u32,
                col: ci as u16,
                cell_ref: cell_ref::format_cell_ref(ri as u32, ci as u16),
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

pub(crate) fn classify_diff(old_cell: Option<&CellData>, new_cell: Option<&CellData>) -> DiffType {
    match (old_cell, new_cell) {
        (None, Some(_)) => DiffType::Add,
        (Some(_), None) => DiffType::Delete,
        (Some(a), Some(b)) => {
            if a.formula != b.formula {
                DiffType::Modify
            } else if a.value != b.value && a.formula.is_some() {
                DiffType::Passive
            } else if a.value != b.value {
                DiffType::Modify
            } else {
                DiffType::NoChange
            }
        }
        (None, None) => DiffType::NoChange,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use excel_core::types::CellDataType;

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
}
