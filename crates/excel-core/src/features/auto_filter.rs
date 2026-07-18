//! AutoFilter feature implementation.
//!
//! Provides Excel native AutoFilter support:
//! - set_auto_filter: apply an autofilter range
//! - remove_auto_filter: clear the autofilter
//! - get_auto_filter: read the current autofilter state
//!
//! Uses rust_xlsxwriter's `worksheet.autofilter()` for writing,
//! and reads the worksheet XML via `zip` for detection.

use std::io::Read;

use crate::security;
use crate::types::*;
use crate::utils::cell_ref;

/// Set an autofilter range on a worksheet.
///
/// The range must include the header row, e.g. "A1:D100".
/// Excel only supports one autofilter per worksheet.
pub fn set_auto_filter(
    path: &str,
    config: &AutoFilterConfig,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    // Parse the range to get 0-indexed coordinates
    let (r1, c1, r2, c2) = cell_ref::parse_range(&config.range)?;

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

        ws.autofilter(r1, c1, r2, c2).map_err(AppError::Xlsx)?;

        Ok(())
    })?;

    let new_hash = security::compute_file_hash(path)
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    Ok(WriteResult {
        success: true,
        message: format!(
            "Set autofilter on sheet '{}' with range '{}'",
            config.sheet, config.range
        ),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

/// Remove the autofilter from a worksheet.
///
/// Works by rebuilding the worksheet without calling `autofilter()`.
pub fn remove_auto_filter(path: &str, sheet: &str, params: &SecurityParams) -> Result<WriteResult> {
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
        let sheet_position = sheet_names
            .iter()
            .position(|n| *n == sheet)
            .ok_or_else(|| AppError::SheetNotFound(sheet.to_string()))?;

        for name in &sheet_names {
            let sd = &old_data[*name];
            let ws = wb.add_worksheet();
            ws.set_name(*name).map_err(AppError::Xlsx)?;
            crate::excel_write::write_sheet_data(ws, sd)?;
        }
        // The worksheet is rebuilt without calling autofilter(), so no autofilter remains.
        let _ = sheet_position;

        Ok(())
    })?;

    let new_hash = security::compute_file_hash(path)
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    Ok(WriteResult {
        success: true,
        message: format!("Removed autofilter from sheet '{}'", sheet),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

/// Read the current autofilter state of a worksheet.
///
/// Inspects the worksheet XML inside the xlsx archive to detect
/// the `<autoFilter>` element.
pub fn get_auto_filter(path: &str, sheet: &str) -> Result<AutoFilterInfo> {
    let file = std::fs::File::open(path).map_err(AppError::Io)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::Read(format!("Failed to open xlsx archive: {}", e)))?;

    // Find the worksheet XML file for the target sheet
    // First, read the workbook relationships to find sheet ID -> file mapping
    let mut sheet_filename = None;

    // Try reading workbook.xml to find the sheet file mapping
    if let Ok(mut wb_xml) = archive.by_name("xl/workbook.xml") {
        let mut xml_data = String::new();
        if wb_xml.read_to_string(&mut xml_data).is_ok() {
            // Extract sheet name to file mapping
            // Format: <sheet name="Sheet1" sheetId="1" r:id="rId1"/>
            let mut current_sheet_name = None;
            for line in xml_data.lines() {
                if line.contains("<sheet ") {
                    if let Some(start) = line.find("name=\"") {
                        let rest = &line[start + 6..];
                        if let Some(end) = rest.find('"') {
                            current_sheet_name = Some(rest[..end].to_string());
                        }
                    }
                    if current_sheet_name.as_deref() == Some(sheet) {
                        if let Some(start) = line.find("sheetId=\"") {
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
    }

    // If we couldn't find via workbook.xml, try the standard naming
    let sheet_filename = sheet_filename.unwrap_or_else(|| format!("xl/worksheets/sheet1.xml"));

    // Read the worksheet XML and check for <autoFilter>
    let enabled = match archive.by_name(&sheet_filename) {
        Ok(mut ws_xml) => {
            let mut xml_data = String::new();
            if ws_xml.read_to_string(&mut xml_data).is_ok() {
                xml_data.contains("<autoFilter ")
            } else {
                false
            }
        }
        Err(_) => false,
    };

    // Extract range if autofilter is enabled
    let range = if enabled {
        // Try to extract the ref attribute from <autoFilter ref="A1:D100">
        let file = std::fs::File::open(path).map_err(AppError::Io)?;
        let mut archive2 = zip::ZipArchive::new(file)
            .map_err(|e| AppError::Read(format!("Failed to open xlsx archive: {}", e)))?;
        if let Ok(mut ws_xml) = archive2.by_name(&sheet_filename) {
            let mut xml_data = String::new();
            if ws_xml.read_to_string(&mut xml_data).is_ok() {
                extract_autofilter_range(&xml_data)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    Ok(AutoFilterInfo {
        sheet: sheet.to_string(),
        range,
        enabled,
    })
}

/// Extract the autofilter range from worksheet XML.
fn extract_autofilter_range(xml: &str) -> Option<String> {
    let tag_start = xml.find("<autoFilter ")?;
    let tag_end = xml[tag_start..].find("/>")?;
    let tag = &xml[tag_start..tag_start + tag_end + 2];

    // Extract ref="A1:D100" or ref='A1:D100'
    if let Some(ref_start) = tag.find("ref=\"") {
        let rest = &tag[ref_start + 5..];
        if let Some(ref_end) = rest.find('"') {
            return Some(rest[..ref_end].to_string());
        }
    }
    if let Some(ref_start) = tag.find("ref='") {
        let rest = &tag[ref_start + 5..];
        if let Some(ref_end) = rest.find('\'') {
            return Some(rest[..ref_end].to_string());
        }
    }
    None
}
