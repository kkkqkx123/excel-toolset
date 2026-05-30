use std::collections::HashMap;

use rust_xlsxwriter::{Workbook, Worksheet};

use crate::excel_diff::diff_sheet_maps;
use crate::excel_read::read_all_sheets_to_map;
use crate::security::{compute_file_hash, create_backup};
use crate::types::*;

// ---------------------------------------------------------------------------
// Row operations
// ---------------------------------------------------------------------------

pub fn append_rows(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    data: &[Vec<CellValue>],
) -> Result<WriteResult> {
    modify_data_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::Custom(format!("Sheet '{}' not found", sheet)))?;

        let new_rows: Vec<Vec<CellData>> = data
            .iter()
            .map(|row| row.iter().map(cell_value_to_data).collect())
            .collect();

        sd.rows.extend(new_rows);
        Ok(new_data)
    })
}

pub fn insert_rows(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    at_row: u32,
    data: &[Vec<CellValue>],
) -> Result<WriteResult> {
    modify_data_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::Custom(format!("Sheet '{}' not found", sheet)))?;

        let new_rows: Vec<Vec<CellData>> = data
            .iter()
            .map(|row| row.iter().map(cell_value_to_data).collect())
            .collect();

        let idx = at_row as usize;
        let mut tail = sd.rows.split_off(idx);
        sd.rows.extend(new_rows);
        sd.rows.append(&mut tail);
        Ok(new_data)
    })
}

pub fn delete_rows(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    start_row: u32,
    end_row: u32,
) -> Result<WriteResult> {
    modify_data_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::Custom(format!("Sheet '{}' not found", sheet)))?;

        let start = start_row as usize;
        let end = end_row as usize;
        if start < sd.rows.len() {
            let count = (end.saturating_sub(start) + 1).min(sd.rows.len() - start);
            for _ in 0..count {
                sd.rows.remove(start);
            }
        }
        Ok(new_data)
    })
}

// ---------------------------------------------------------------------------
// Query operations
// ---------------------------------------------------------------------------

pub fn filter_rows(
    path: &str,
    sheet: &str,
    conditions: &[FilterCondition],
) -> Result<Vec<Vec<CellData>>> {
    let data = crate::excel_read::read_sheet_all(path, sheet)?;
    let header = data.rows.first().cloned().unwrap_or_default();
    let mut results = vec![header];

    for row in data.rows.iter().skip(1) {
        if matches_all(row, conditions) {
            results.push(row.clone());
        }
    }
    Ok(results)
}

pub fn sort_sheet(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    sort_columns: &[SortColumn],
) -> Result<WriteResult> {
    modify_data_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::Custom(format!("Sheet '{}' not found", sheet)))?;

        if sd.rows.len() > 1 {
            let header = sd.rows[0].clone();
            let mut body: Vec<Vec<CellData>> = sd.rows.drain(1..).collect();

            body.sort_by(|a, b| {
                for sc in sort_columns {
                    let ca = a.get(sc.column as usize).and_then(|c| c.value.as_deref()).unwrap_or("");
                    let cb = b.get(sc.column as usize).and_then(|c| c.value.as_deref()).unwrap_or("");
                    let cmp = ca.to_lowercase().cmp(&cb.to_lowercase());
                    if cmp != std::cmp::Ordering::Equal {
                        return if sc.descending { cmp.reverse() } else { cmp };
                    }
                }
                std::cmp::Ordering::Equal
            });

            sd.rows.push(header);
            sd.rows.extend(body);
        }
        Ok(new_data)
    })
}

pub fn dedup_sheet(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    columns: &[u16],
) -> Result<WriteResult> {
    modify_data_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::Custom(format!("Sheet '{}' not found", sheet)))?;

        if sd.rows.len() > 1 {
            let header = sd.rows[0].clone();
            let body: Vec<Vec<CellData>> = sd.rows.drain(1..).collect();
            let mut seen = std::collections::HashSet::new();
            let cols: Vec<usize> = if columns.is_empty() {
                (0..body.iter().map(|r| r.len()).max().unwrap_or(0)).collect()
            } else {
                columns.iter().map(|c| *c as usize).collect()
            };

            for row in body {
                let key: Vec<String> = cols
                    .iter()
                    .map(|&ci| row.get(ci).and_then(|c| c.value.as_deref()).unwrap_or("").to_string())
                    .collect();
                if seen.insert(key) {
                    sd.rows.push(row);
                }
            }
            sd.rows.insert(0, header);
        }
        Ok(new_data)
    })
}

pub fn sql_query(path: &str, sheet: &str, _sql: &str) -> Result<Vec<Vec<CellData>>> {
    let data = crate::excel_read::read_sheet_all(path, sheet)?;
    Ok(data.rows)
}

// ---------------------------------------------------------------------------
// Internal
// ---------------------------------------------------------------------------

fn modify_data_file<F>(path: &str, params: &SecurityParams, modifier: F) -> Result<WriteResult>
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

    let sheet_diffs = diff_sheet_maps(&old_data, &new_data);
    let flat_diffs: Vec<CellDiff> = sheet_diffs
        .iter()
        .flat_map(|sd| sd.cell_diffs.clone())
        .collect();
    let diff = if flat_diffs.is_empty() { None } else { Some(flat_diffs) };

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff,
    })
}

fn write_sheet_data(ws: &mut Worksheet, data: &SheetData) -> Result<()> {
    for (ri, row) in data.rows.iter().enumerate() {
        for (ci, cell) in row.iter().enumerate() {
            if let Some(ref formula) = cell.formula {
                ws.write_formula(ri as u32, ci as u16, formula.as_str())
                    .map_err(AppError::Xlsx)?;
            } else if let Some(ref val) = cell.value {
                match cell.data_type {
                    CellDataType::Float | CellDataType::Int | CellDataType::DateTime => {
                        if let Ok(n) = val.parse::<f64>() {
                            ws.write_number(ri as u32, ci as u16, n).map_err(AppError::Xlsx)?;
                        } else {
                            ws.write_string(ri as u32, ci as u16, val).map_err(AppError::Xlsx)?;
                        }
                    }
                    CellDataType::Bool => {
                        let b = val == "true" || val == "1" || val == "True";
                        ws.write_boolean(ri as u32, ci as u16, b).map_err(AppError::Xlsx)?;
                    }
                    _ => {
                        ws.write_string(ri as u32, ci as u16, val).map_err(AppError::Xlsx)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn cell_value_to_data(val: &CellValue) -> CellData {
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
        CellValue::Empty => CellData {
            value: None,
            data_type: CellDataType::Empty,
            formula: None,
        },
    }
}

fn matches_all(row: &[CellData], conditions: &[FilterCondition]) -> bool {
    conditions.iter().all(|c| matches_one(row, c))
}

fn matches_one(row: &[CellData], cond: &FilterCondition) -> bool {
    let cell_val = row
        .get(cond.column as usize)
        .and_then(|c| c.value.as_deref())
        .unwrap_or("");
    let lower_val = cell_val.to_lowercase();
    let lower_cond = cond.value.to_lowercase();

    match cond.operator {
        FilterOp::Eq => lower_val == lower_cond,
        FilterOp::Ne => lower_val != lower_cond,
        FilterOp::Gt => lower_val > lower_cond,
        FilterOp::Lt => lower_val < lower_cond,
        FilterOp::Ge => lower_val >= lower_cond,
        FilterOp::Le => lower_val <= lower_cond,
        FilterOp::Contains => lower_val.contains(&lower_cond),
        FilterOp::StartsWith => lower_val.starts_with(&lower_cond),
        FilterOp::EndsWith => lower_val.ends_with(&lower_cond),
    }
}
