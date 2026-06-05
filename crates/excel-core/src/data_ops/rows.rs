use crate::types::*;

use super::modify::{cell_value_to_data, modify_data_file};

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
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

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
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

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
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

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