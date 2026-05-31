use std::collections::HashMap;

use crate::cell_ref;
use crate::types::*;

use super::data::{cell_value_to_data, ensure_dimensions};

pub fn write(
    data: &mut HashMap<String, SheetData>,
    sheet: &str,
    row: u32,
    col: u16,
    value: &CellValue,
) -> Result<()> {
    let sd = data
        .get_mut(sheet)
        .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;
    ensure_dimensions(sd, row as usize, col as usize);
    sd.rows[row as usize][col as usize] = cell_value_to_data(value);
    Ok(())
}

pub fn write_range(
    data: &mut HashMap<String, SheetData>,
    sheet: &str,
    range: &str,
    grid: &[Vec<CellValue>],
) -> Result<()> {
    let (r1, c1, _, _) = cell_ref::parse_range(range)?;
    let sd = data
        .get_mut(sheet)
        .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;
    for (ri, row) in grid.iter().enumerate() {
        for (ci, val) in row.iter().enumerate() {
            let target_row = r1 as usize + ri;
            let target_col = c1 as usize + ci;
            ensure_dimensions(sd, target_row, target_col);
            sd.rows[target_row][target_col] = cell_value_to_data(val);
        }
    }
    Ok(())
}

pub fn clear_range(data: &mut HashMap<String, SheetData>, sheet: &str, range: &str) -> Result<()> {
    let (r_start, r_end, c_start, c_end) = cell_ref::parse_range_normalized(range)?;
    let sd = data
        .get_mut(sheet)
        .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;
    for ri in r_start..=r_end {
        for ci in c_start..=c_end {
            let row = ri as usize;
            let col = ci as usize;
            if row < sd.rows.len() && col < sd.rows[row].len() {
                sd.rows[row][col] = CellData {
                    value: None,
                    data_type: CellDataType::Empty,
                    formula: None,
                };
            }
        }
    }
    Ok(())
}

pub fn set_formula(
    data: &mut HashMap<String, SheetData>,
    sheet: &str,
    cell: &str,
    formula: &str,
) -> Result<()> {
    let (row, col) = cell_ref::parse_cell_ref(cell)?;
    let sd = data
        .get_mut(sheet)
        .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;
    ensure_dimensions(sd, row as usize, col as usize);
    sd.rows[row as usize][col as usize] = CellData {
        value: None,
        data_type: CellDataType::String,
        formula: Some(formula.to_string()),
    };
    Ok(())
}

pub fn insert_rows(
    data: &mut HashMap<String, SheetData>,
    sheet: &str,
    at_row: u32,
    grid: &[Vec<CellValue>],
) -> Result<()> {
    let sd = data
        .get_mut(sheet)
        .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;
    let new_rows: Vec<Vec<CellData>> = grid
        .iter()
        .map(|row| row.iter().map(cell_value_to_data).collect())
        .collect();
    let idx = at_row as usize;
    let mut tail = sd.rows.split_off(idx);
    sd.rows.extend(new_rows);
    sd.rows.append(&mut tail);
    Ok(())
}

pub fn delete_rows(
    data: &mut HashMap<String, SheetData>,
    sheet: &str,
    start_row: u32,
    end_row: u32,
) -> Result<()> {
    let sd = data
        .get_mut(sheet)
        .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;
    let start = start_row as usize;
    let end = end_row as usize;
    if start < sd.rows.len() {
        let count = (end.saturating_sub(start) + 1).min(sd.rows.len() - start);
        for _ in 0..count {
            sd.rows.remove(start);
        }
    }
    Ok(())
}

pub fn append_rows(
    data: &mut HashMap<String, SheetData>,
    sheet: &str,
    grid: &[Vec<CellValue>],
) -> Result<()> {
    let sd = data
        .get_mut(sheet)
        .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;
    let new_rows: Vec<Vec<CellData>> = grid
        .iter()
        .map(|row| row.iter().map(cell_value_to_data).collect())
        .collect();
    sd.rows.extend(new_rows);
    Ok(())
}
