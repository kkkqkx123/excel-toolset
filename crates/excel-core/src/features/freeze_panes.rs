//! Freeze panes feature implementation.
//!
//! Keeps specified rows and/or columns visible while scrolling the worksheet.
//! Supports row freezing, column freezing, or both simultaneously.

use crate::security;
use crate::types::*;

/// Set freeze panes on a worksheet.
///
/// `rows` is the number of rows to freeze from the top (0 = no row freeze).
/// `cols` is the number of columns to freeze from the left (0 = no column freeze).
pub fn set_freeze_panes(
    path: &str,
    config: &FreezePanesConfig,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    let backup_info = security::create_backup_if_needed(params)
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    let old_hash = security::compute_file_hash(path)
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    crate::excel_write::modify_file_with_wb(path, params, |old_data, wb| {
        *wb = rust_xlsxwriter::Workbook::new();

        let sheet_names: Vec<&str> = old_data.keys().map(|s| s.as_str()).collect();
        for name in &sheet_names {
            let sd = &old_data[*name];
            let ws = wb.add_worksheet();
            ws.set_name(*name).map_err(AppError::Xlsx)?;
            crate::excel_write::write_sheet_data(ws, sd)?;
        }

        let sheet_idx = sheet_names
            .iter()
            .position(|n| *n == config.sheet)
            .ok_or_else(|| AppError::SheetNotFound(config.sheet.clone()))?;

        let ws = wb
            .worksheet_from_index(sheet_idx)
            .map_err(|_e| AppError::SheetNotFound(config.sheet.clone()))?;

        ws.set_freeze_panes(config.rows, config.cols)
            .map_err(AppError::Xlsx)?;

        Ok(())
    })?;

    let new_hash = security::compute_file_hash(path)
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    Ok(WriteResult {
        success: true,
        message: format!(
            "Set freeze panes on sheet '{}': rows={}, cols={}",
            config.sheet, config.rows, config.cols
        ),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

/// Clear freeze panes from a worksheet.
pub fn clear_freeze_panes(path: &str, sheet: &str, params: &SecurityParams) -> Result<WriteResult> {
    let config = FreezePanesConfig {
        sheet: sheet.to_string(),
        rows: 0,
        cols: 0,
    };
    set_freeze_panes(path, &config, params)
}
