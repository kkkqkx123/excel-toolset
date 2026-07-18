//! Page setup feature implementation.
//!
//! Configures worksheet print properties: orientation, paper size, margins,
//! print area, print titles, scaling, gridlines, headings, centering, and
//! page breaks. Uses rust_xlsxwriter's worksheet-level page setup APIs.

use crate::security;
use crate::types::*;

/// Configure page setup settings on a worksheet.
///
/// Applies all non-None fields from the config. Fields left as None
/// are not modified from defaults.
pub fn configure_page_setup(
    path: &str,
    config: &PageSetupConfig,
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

        // Apply page orientation
        if let Some(ref orientation) = config.orientation {
            match orientation {
                PageOrientation::Portrait => {
                    ws.set_portrait();
                }
                PageOrientation::Landscape => {
                    ws.set_landscape();
                }
            }
        }

        // Apply paper size
        if let Some(ref paper_size) = config.paper_size {
            ws.set_paper_size(paper_size.to_xlsx_index());
        }

        // Apply margins
        if let Some(ref margins) = config.margins {
            ws.set_margins(
                margins.left,
                margins.right,
                margins.top,
                margins.bottom,
                margins.header,
                margins.footer,
            );
        }

        // Apply print area
        if let Some(ref range) = config.print_area {
            let (r1, c1, r2, c2) = crate::utils::cell_ref::parse_range(range)?;
            ws.set_print_area(r1, c1, r2, c2)
                .map_err(AppError::Xlsx)?;
        }

        // Apply print title rows
        if let Some(ref rows_str) = config.print_title_rows {
            let parts: Vec<&str> = rows_str.split(':').collect();
            if parts.len() == 2 {
                let first: u32 = parts[0].parse().map_err(|_| {
                    AppError::InvalidInput(format!("Invalid print title rows: {}", rows_str))
                })?;
                let last: u32 = parts[1].parse().map_err(|_| {
                    AppError::InvalidInput(format!("Invalid print title rows: {}", rows_str))
                })?;
                // Convert to 0-indexed for rust_xlsxwriter
                ws.set_repeat_rows(first.saturating_sub(1), last.saturating_sub(1))
                    .map_err(AppError::Xlsx)?;
            } else {
                return Err(AppError::InvalidInput(format!(
                    "Print title rows must be in format 'first:last', got: {}",
                    rows_str
                )));
            }
        }

        // Apply print title columns
        if let Some(ref cols_str) = config.print_title_cols {
            let parts: Vec<&str> = cols_str.split(':').collect();
            if parts.len() == 2 {
                let first = crate::utils::cell_ref::col_to_index(parts[0])?;
                let last = crate::utils::cell_ref::col_to_index(parts[1])?;
                ws.set_repeat_columns(first, last)
                    .map_err(AppError::Xlsx)?;
            } else {
                return Err(AppError::InvalidInput(format!(
                    "Print title columns must be in format 'A:B', got: {}",
                    cols_str
                )));
            }
        }

        // Apply fit-to-pages or scale (fit_to_pages takes priority)
        if let Some(ref fit) = config.fit_to_pages {
            ws.set_print_fit_to_pages(fit.width, fit.height);
        } else if let Some(scale) = config.scale {
            ws.set_print_scale(scale);
        }

        // Apply gridlines
        if config.print_gridlines {
            ws.set_print_gridlines(true);
        }

        // Apply headings
        if config.print_headings {
            ws.set_print_headings(true);
        }

        // Apply centering
        if config.center_horizontally {
            ws.set_print_center_horizontally(true);
        }
        if config.center_vertically {
            ws.set_print_center_vertically(true);
        }

        Ok(())
    })?;

    let new_hash = security::compute_file_hash(path)
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    Ok(WriteResult {
        success: true,
        message: format!(
            "Configured page setup on sheet '{}'",
            config.sheet
        ),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

/// Set horizontal and vertical page breaks on a worksheet.
pub fn set_page_breaks(
    path: &str,
    config: &PageBreakConfig,
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

        if !config.horizontal_breaks.is_empty() {
            ws.set_page_breaks(&config.horizontal_breaks);
        }
        if !config.vertical_breaks.is_empty() {
            let vbreaks: Vec<u32> = config.vertical_breaks.iter().map(|&v| v as u32).collect();
            ws.set_vertical_page_breaks(&vbreaks);
        }

        Ok(())
    })?;

    let new_hash = security::compute_file_hash(path)
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    Ok(WriteResult {
        success: true,
        message: format!(
            "Set page breaks on sheet '{}': {} horizontal, {} vertical",
            config.sheet,
            config.horizontal_breaks.len(),
            config.vertical_breaks.len()
        ),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

/// Clear all page breaks from a worksheet.
pub fn clear_page_breaks(
    path: &str,
    sheet: &str,
    params: &SecurityParams,
) -> Result<WriteResult> {
    let config = PageBreakConfig {
        sheet: sheet.to_string(),
        horizontal_breaks: Vec::new(),
        vertical_breaks: Vec::new(),
    };
    set_page_breaks(path, &config, params)
}
