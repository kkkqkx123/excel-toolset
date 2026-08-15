//! Sheet protection feature implementation.
//!
//! Provides worksheet protection support:
//! - protect_sheet: protect a worksheet with optional password and options
//! - unprotect_sheet: remove protection from a worksheet
//! - is_sheet_protected: check if a worksheet is protected
//!
//! Uses rust_xlsxwriter's `worksheet.protect()` / `worksheet.protect_with_password()`
//! / `worksheet.protect_with_options()` for writing, and reads the worksheet XML
//! via `zip` for detection.

use std::io::Read;

use crate::types::*;

/// Protect a worksheet with optional password and protection options.
pub fn protect_sheet(
    path: &str,
    config: &SheetProtectionConfig,
    params: &SecurityParams,
) -> Result<WriteResult> {
    #[cfg(feature = "zip")]
    {
        crate::excel_write::patch::protect_sheet_preserving(path, params, &config.sheet, config)
    }

    #[cfg(not(feature = "zip"))]
    {
        if params.dry_run {
            return Ok(WriteResult::dry_run_success());
        }

        let backup_info = security::create_backup_if_needed(params)
            .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        let old_hash = security::compute_file_hash(path)
            .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        let _ = security::create_backup_if_needed(params)
            .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        crate::excel_write::modify_file_with_wb(path, params, |_old_data, wb| {
            let ws = wb
                .worksheet_from_name(&config.sheet)
                .map_err(|_e| AppError::SheetNotFound(config.sheet.clone()))?;

            let protection_options = rust_xlsxwriter::ProtectionOptions {
                select_locked_cells: config.options.select_locked_cells,
                select_unlocked_cells: config.options.select_unlocked_cells,
                format_cells: config.options.format_cells,
                format_columns: config.options.format_columns,
                format_rows: config.options.format_rows,
                insert_rows: config.options.insert_rows,
                insert_columns: config.options.insert_columns,
                insert_links: config.options.insert_links,
                delete_rows: config.options.delete_rows,
                delete_columns: config.options.delete_columns,
                sort: config.options.sort,
                use_autofilter: config.options.auto_filter,
                use_pivot_tables: config.options.pivot_tables,
                edit_scenarios: config.options.edit_scenarios,
                edit_objects: config.options.edit_objects,
                contents: config.options.contents,
            };

            match &config.password {
                Some(pwd) if !pwd.is_empty() => {
                    ws.protect_with_password(pwd);
                }
                _ => {
                    ws.protect();
                }
            }

            ws.protect_with_options(&protection_options);

            Ok(())
        })?;

        let new_hash = security::compute_file_hash(path)
            .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        Ok(WriteResult {
            success: true,
            message: format!(
                "Protected sheet '{}'{}",
                config.sheet,
                if config.password.is_some() {
                    " with password"
                } else {
                    ""
                }
            ),
            backup_info,
            old_hash,
            new_hash,
            diff: None,
        })
    }
}

/// Remove protection from a worksheet.
pub fn unprotect_sheet(path: &str, sheet: &str, params: &SecurityParams) -> Result<WriteResult> {
    #[cfg(feature = "zip")]
    {
        crate::excel_write::patch::unprotect_sheet_preserving(path, params, sheet)
    }

    #[cfg(not(feature = "zip"))]
    {
        if params.dry_run {
            return Ok(WriteResult::dry_run_success());
        }

        let backup_info = security::create_backup_if_needed(params)
            .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        let old_hash = security::compute_file_hash(path)
            .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        crate::excel_write::modify_file_with_wb(path, params, |_old_data, wb| {
            // Rebuilding without protect() drops any existing sheet protection.
            wb.worksheet_from_name(sheet)
                .map_err(|_e| AppError::SheetNotFound(sheet.to_string()))?;
            Ok(())
        })?;

        let new_hash = security::compute_file_hash(path)
            .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        Ok(WriteResult {
            success: true,
            message: format!("Unprotected sheet '{}'", sheet),
            backup_info,
            old_hash,
            new_hash,
            diff: None,
        })
    }
}

/// Check if a worksheet is protected.
///
/// Reads the worksheet XML inside the xlsx archive to detect
/// the `<sheetProtection>` element.
pub fn is_sheet_protected(path: &str, sheet: &str) -> Result<bool> {
    let file = std::fs::File::open(path).map_err(AppError::Io)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::Read(format!("Failed to open xlsx archive: {}", e)))?;

    // Find the worksheet XML file for the target sheet
    let mut sheet_filename = None;

    if let Ok(mut wb_xml) = archive.by_name("xl/workbook.xml") {
        let mut xml_data = String::new();
        if wb_xml.read_to_string(&mut xml_data).is_ok() {
            let mut current_sheet_name = None;
            for line in xml_data.lines() {
                if line.contains("<sheet ") {
                    if let Some(start) = line.find("name=\"") {
                        let rest = &line[start + 6..];
                        if let Some(end) = rest.find('"') {
                            current_sheet_name = Some(rest[..end].to_string());
                        }
                    }
                    if current_sheet_name.as_deref() == Some(sheet)
                        && let Some(start) = line.find("sheetId=\"") {
                            let rest = &line[start + 9..];
                            if let Some(end) = rest.find('"') {
                                let sheet_id: String = rest[..end].to_string();
                                sheet_filename =
                                    Some(format!("xl/worksheets/sheet{}.xml", sheet_id));
                                break;
                            }
                        }
                }
            }
        }
    }

    let sheet_filename = sheet_filename.unwrap_or_else(|| "xl/worksheets/sheet1.xml".to_string());

    match archive.by_name(&sheet_filename) {
        Ok(mut ws_xml) => {
            let mut xml_data = String::new();
            if ws_xml.read_to_string(&mut xml_data).is_ok() {
                Ok(xml_data.contains("<sheetProtection "))
            } else {
                Ok(false)
            }
        }
        Err(_) => Ok(false),
    }
}
