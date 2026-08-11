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

    crate::excel_write::modify_file_with_wb(path, params, |_old_data, wb| {
        // Do NOT rebuild the workbook here: `modify_file_with_wb` has already
        // recreated every worksheet in the file's original order. Rebuilding
        // from a HashMap shuffled the tabs randomly per run and made the
        // visibility flag land on the wrong tab.
        let ws = wb
            .worksheet_from_name(sheet)
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
