use std::collections::HashMap;

use rust_xlsxwriter::{Workbook, Worksheet};

use crate::excel_read::read_all_sheets_to_map;
use crate::security::{compute_file_hash, create_backup};
use crate::types::*;

pub fn modify_data_file<F>(path: &str, params: &SecurityParams, modifier: F) -> Result<WriteResult>
where
    F: FnOnce(&HashMap<String, SheetData>) -> Result<HashMap<String, SheetData>>,
{
    let old_hash = compute_file_hash(path).map_err(AppError::Io)?;
    let backup_info = if params.create_backup {
        Some(create_backup(path, &old_hash).map_err(AppError::Io)?)
    } else {
        None
    };

    let old_data = read_all_sheets_to_map(path)?;
    let new_data = modifier(&old_data)?;

    let mut wb = Workbook::new();
    for (name, data) in &new_data {
        let ws = wb.add_worksheet();
        ws.set_name(name).map_err(AppError::Xlsx)?;
        write_sheet_data(ws, data)?;
    }

    let new_hash = if params.dry_run {
        old_hash.clone()
    } else {
        wb.save(path).map_err(AppError::Xlsx)?;
        compute_file_hash(path).map_err(AppError::Io)?
    };

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

pub(crate) fn write_sheet_data(ws: &mut Worksheet, data: &SheetData) -> Result<()> {
    for (ri, row) in data.rows.iter().enumerate() {
        for (ci, cell) in row.iter().enumerate() {
            if let Some(ref formula) = cell.formula {
                ws.write_formula(ri as u32, ci as u16, formula.as_str())
                    .map_err(AppError::Xlsx)?;
            } else if let Some(ref val) = cell.value {
                match cell.data_type {
                    CellDataType::Float | CellDataType::Int | CellDataType::DateTime => {
                        if let Ok(n) = val.parse::<f64>() {
                            ws.write_number(ri as u32, ci as u16, n)
                                .map_err(AppError::Xlsx)?;
                        } else {
                            ws.write_string(ri as u32, ci as u16, val)
                                .map_err(AppError::Xlsx)?;
                        }
                    }
                    CellDataType::Bool => {
                        let b = val == "true" || val == "1" || val == "True";
                        ws.write_boolean(ri as u32, ci as u16, b)
                            .map_err(AppError::Xlsx)?;
                    }
                    _ => {
                        ws.write_string(ri as u32, ci as u16, val)
                            .map_err(AppError::Xlsx)?;
                    }
                }
            }
        }
    }
    Ok(())
}

pub(super) fn cell_value_to_data(val: &CellValue) -> CellData {
    match val {
        CellValue::String(s) => CellData {
            value: Some(s.clone()),
            data_type: CellDataType::String,
            formula: None,
        },
        CellValue::Number(n) => CellData {
            value: Some(n.to_string()),
            data_type: CellDataType::Float,
            formula: None,
        },
        CellValue::Bool(b) => CellData {
            value: Some(b.to_string()),
            data_type: CellDataType::Bool,
            formula: None,
        },
        CellValue::DateTime(dt) => CellData {
            value: Some(dt.to_string()),
            data_type: CellDataType::DateTime,
            formula: None,
        },
        CellValue::Error(e) => CellData {
            value: Some(e.clone()),
            data_type: CellDataType::Error,
            formula: None,
        },
        CellValue::Empty => CellData {
            value: None,
            data_type: CellDataType::Empty,
            formula: None,
        },
    }
}