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
