use std::collections::HashMap;

use calamine::{Data, Reader, Xlsx, open_workbook};

use crate::security::compute_file_hash;
use crate::types::{AppError, CellData, CellDataType, FileInfo, Result, SheetData};
use crate::utils::cell_ref;

pub fn read_file_info(path: &str) -> Result<FileInfo> {
    let sheets = list_sheets(path)?;
    let hash = compute_file_hash(path).map_err(AppError::Io)?;
    let size = std::fs::metadata(path)
        .map(|m| m.len())
        .map_err(AppError::Io)?;

    Ok(FileInfo {
        path: path.to_string(),
        hash,
        size,
        sheets,
        created_at: chrono::Utc::now(),
    })
}

pub fn list_sheets(path: &str) -> Result<Vec<String>> {
    let workbook: Xlsx<_> = open_workbook(path)?;
    Ok(workbook.sheet_names().to_vec())
}

pub fn read_cell(path: &str, sheet: &str, row: u32, col: u16) -> Result<CellData> {
    let mut workbook: Xlsx<_> = open_workbook(path)?;

    let range = workbook.worksheet_range(sheet)?;

    let ws_formulas = workbook.worksheet_formula(sheet).ok();

    let cell = range
        .get_value((row, col as u32))
        .unwrap_or(&calamine::Data::Empty);

    let formula = ws_formulas
        .as_ref()
        .and_then(|f| f.get_value((row, col as u32)).map(|s| s.to_string()));

    Ok(data_to_cell_data(cell, formula))
}

pub fn read_range(path: &str, sheet: &str, range_spec: &str) -> Result<Vec<Vec<CellData>>> {
    let (r_start, r_end, c_start, c_end) = cell_ref::parse_range_normalized(range_spec)?;
    let mut workbook: Xlsx<_> = open_workbook(path)?;

    let range = workbook.worksheet_range(sheet)?;

    let ws_formulas = workbook.worksheet_formula(sheet).ok();

    let mut result = Vec::new();
    for row in r_start..=r_end {
        let mut row_data = Vec::new();
        for col in c_start..=c_end {
            let cell = range.get_value((row, col as u32)).unwrap_or(&Data::Empty);
            let formula = ws_formulas
                .as_ref()
                .and_then(|f| f.get_value((row, col as u32)).map(|s| s.to_string()));
            row_data.push(data_to_cell_data(cell, formula));
        }
        result.push(row_data);
    }

    Ok(result)
}

pub fn read_formula(path: &str, sheet: &str, cell_spec: &str) -> Result<Option<String>> {
    let (row, col) = cell_ref::parse_cell_ref(cell_spec)?;
    let mut workbook: Xlsx<_> = open_workbook(path)?;

    let formulas = workbook.worksheet_formula(sheet)?;

    Ok(formulas.get_value((row, col as u32)).map(|s| {
        let formula = s.to_string();
        // Add = prefix if not present, as calamine stores formulas without it
        if formula.starts_with('=') {
            formula
        } else {
            format!("={}", formula)
        }
    }))
}

pub fn read_sheet_all(path: &str, sheet: &str) -> Result<SheetData> {
    let mut workbook: Xlsx<_> = open_workbook(path)?;

    let range = workbook.worksheet_range(sheet)?;

    let ws_formulas = workbook.worksheet_formula(sheet).ok();

    let mut rows = Vec::new();
    for (row_idx, row) in range.rows().enumerate() {
        let mut cells = Vec::new();
        for (col_idx, cell) in row.iter().enumerate() {
            let formula = ws_formulas.as_ref().and_then(|f| {
                f.get_value((row_idx as u32, col_idx as u32))
                    .map(|s| s.to_string())
            });
            cells.push(data_to_cell_data(cell, formula));
        }
        rows.push(cells);
    }

    Ok(SheetData {
        name: sheet.to_string(),
        rows,
    })
}

pub(crate) fn read_all_sheets_to_map(path: &str) -> Result<HashMap<String, SheetData>> {
    let sheets = list_sheets(path)?;
    let mut map = HashMap::new();
    for name in sheets {
        let data = read_sheet_all(path, &name)?;
        map.insert(name, data);
    }
    Ok(map)
}

fn data_to_cell_data(cell: &Data, formula: Option<String>) -> CellData {
    match cell {
        Data::String(s) => CellData {
            value: Some(s.clone()),
            data_type: CellDataType::String,
            formula,
        },
        Data::Float(f) => CellData {
            value: Some(f.to_string()),
            data_type: CellDataType::Float,
            formula,
        },
        Data::Int(i) => CellData {
            value: Some(i.to_string()),
            data_type: CellDataType::Int,
            formula,
        },
        Data::Bool(b) => CellData {
            value: Some(b.to_string()),
            data_type: CellDataType::Bool,
            formula,
        },
        Data::DateTime(f) => CellData {
            value: Some(f.to_string()),
            data_type: CellDataType::DateTime,
            formula,
        },
        Data::DateTimeIso(s) => CellData {
            value: Some(s.clone()),
            data_type: CellDataType::DateTime,
            formula,
        },
        Data::DurationIso(s) => CellData {
            value: Some(s.clone()),
            data_type: CellDataType::String,
            formula,
        },
        Data::Error(e) => CellData {
            value: Some(format!("{}", e)),
            data_type: CellDataType::Error,
            formula,
        },
        Data::Empty => CellData {
            value: None,
            data_type: CellDataType::Empty,
            formula,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_sheets_nonexistent_file() {
        let result = list_sheets("_nonexistent_file.xlsx");
        assert!(result.is_err());
    }
}
