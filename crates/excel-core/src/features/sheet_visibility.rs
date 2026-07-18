//! Sheet visibility feature implementation.
//!
//! Controls worksheet visibility with three levels:
//! - Visible: normal visible sheet
//! - Hidden: hidden but user can unhide via Excel UI
//! - VeryHidden: deeply hidden, can only be unhidden via VBA

use crate::security;
use crate::types::*;

/// Set the visibility of a worksheet.
pub fn set_sheet_visibility(
    path: &str,
    sheet: &str,
    visibility: &SheetVisibility,
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
            .position(|n| *n == sheet)
            .ok_or_else(|| AppError::SheetNotFound(sheet.to_string()))?;

        let ws = wb
            .worksheet_from_index(sheet_idx)
            .map_err(|_e| AppError::SheetNotFound(sheet.to_string()))?;

        match visibility {
            SheetVisibility::Visible => {
                ws.set_hidden(false);
                ws.set_very_hidden(false);
            }
            SheetVisibility::Hidden => {
                ws.set_hidden(true);
            }
            SheetVisibility::VeryHidden => {
                ws.set_very_hidden(true);
            }
        }

        Ok(())
    })?;

    let new_hash = security::compute_file_hash(path)
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    Ok(WriteResult {
        success: true,
        message: format!("Set sheet '{}' visibility to {:?}", sheet, visibility),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}
