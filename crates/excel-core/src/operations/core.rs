use std::collections::HashMap;

use rust_xlsxwriter::Workbook;

use crate::excel_read::read_all_sheets_to_map;
use crate::excel_write::write_sheet_data;
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
