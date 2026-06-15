use crate::types::*;

use super::core::{cell_value_to_data, ensure_dimensions};

pub(crate) fn write(
    data: &mut std::collections::HashMap<String, SheetData>,
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

pub(crate) fn write_range(
    data: &mut std::collections::HashMap<String, SheetData>,
    sheet: &str,
    range_spec: &str,
    grid: &[Vec<CellValue>],
) -> Result<()> {
    let (r1, c1, _, _) = crate::utils::cell_ref::parse_range(range_spec)?;
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

pub(crate) fn clear_range(
    data: &mut std::collections::HashMap<String, SheetData>,
    sheet: &str,
    range_spec: &str,
) -> Result<()> {
    let (r_start, r_end, c_start, c_end) =
        crate::utils::cell_ref::parse_range_normalized(range_spec)?;
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

pub(crate) fn set_formula(
    data: &mut std::collections::HashMap<String, SheetData>,
    sheet: &str,
    cell_spec: &str,
    formula: &str,
) -> Result<()> {
    let (row, col) = crate::utils::cell_ref::parse_cell_ref(cell_spec)?;
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

pub(crate) fn insert_rows(
    data: &mut std::collections::HashMap<String, SheetData>,
    sheet: &str,
    at_row: u32,
    new_rows: &[Vec<CellValue>],
) -> Result<()> {
    let sd = data
        .get_mut(sheet)
        .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

    let row_idx = at_row as usize;
    let mut inserted_rows = Vec::new();

    for row_data in new_rows {
        let mut row = Vec::new();
        for val in row_data {
            row.push(cell_value_to_data(val));
        }
        inserted_rows.push(row);
    }

    sd.rows.splice(row_idx..row_idx, inserted_rows);
    Ok(())
}

pub(crate) fn delete_rows(
    data: &mut std::collections::HashMap<String, SheetData>,
    sheet: &str,
    start_row: u32,
    end_row: u32,
) -> Result<()> {
    let sd = data
        .get_mut(sheet)
        .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

    let start_idx = start_row as usize;
    let end_idx = end_row as usize;

    if start_idx >= sd.rows.len() {
        return Ok(());
    }

    let end_idx = end_idx.min(sd.rows.len() - 1);
    sd.rows.drain(start_idx..=end_idx);
    Ok(())
}

pub(crate) fn append_rows(
    data: &mut std::collections::HashMap<String, SheetData>,
    sheet: &str,
    new_rows: &[Vec<CellValue>],
) -> Result<()> {
    let sd = data
        .get_mut(sheet)
        .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

    for row_data in new_rows {
        let mut row = Vec::new();
        for val in row_data {
            row.push(cell_value_to_data(val));
        }
        sd.rows.push(row);
    }
    Ok(())
}
