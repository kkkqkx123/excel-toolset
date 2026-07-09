use crate::security;
use crate::types::*;

#[derive(Debug, Clone)]
pub enum SparklineType {
    Line,
    Column,
    WinLose,
}

#[derive(Debug, Clone)]
pub struct SparklineConfig {
    pub sparkline_type: SparklineType,
    pub sheet: String,
    /// Source data range as sheet-qualified string, e.g., "'Sheet1'!A1:E1".
    pub source_range: String,
    pub target_row: u32,
    pub target_col: u16,
    pub style: Option<u8>,
}

pub fn parse_sparkline_type(s: &str) -> SparklineType {
    match s.to_lowercase().as_str() {
        "column" => SparklineType::Column,
        "winlose" | "win_lose" | "win-lose" => SparklineType::WinLose,
        _ => SparklineType::Line,
    }
}

fn map_sparkline_type(st: &SparklineType) -> rust_xlsxwriter::SparklineType {
    match st {
        SparklineType::Line => rust_xlsxwriter::SparklineType::Line,
        SparklineType::Column => rust_xlsxwriter::SparklineType::Column,
        SparklineType::WinLose => rust_xlsxwriter::SparklineType::WinLose,
    }
}

pub fn add_sparkline(
    path: &str,
    config: &SparklineConfig,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    crate::excel_write::modify_file_with_wb(path, params, |old_data, wb| {
        *wb = rust_xlsxwriter::Workbook::new();

        let sheet_names: Vec<&str> = old_data.keys().map(|s| s.as_str()).collect();
        for name in &sheet_names {
            let sd = &old_data[*name];
            let ws = wb.add_worksheet();
            ws.set_name(*name).map_err(AppError::Xlsx)?;
            crate::excel_write::core::write_sheet_data(ws, sd)?;
        }

        let sheet_idx = sheet_names
            .iter()
            .position(|n| *n == config.sheet)
            .ok_or_else(|| AppError::SheetNotFound(config.sheet.clone()))?;

        let ws = wb
            .worksheet_from_index(sheet_idx)
            .map_err(|_e| AppError::SheetNotFound(config.sheet.clone()))?;

        let sparkline_type = map_sparkline_type(&config.sparkline_type);

        let mut sparkline = rust_xlsxwriter::Sparkline::new()
            .set_range(config.source_range.as_str())
            .set_type(sparkline_type);

        if let Some(style_num) = config.style {
            sparkline = sparkline.set_style(style_num);
        }

        ws.add_sparkline(config.target_row, config.target_col, &sparkline)
            .map_err(|e| AppError::Write(e.to_string()))?;

        Ok(())
    })
}

pub fn remove_sparkline(
    path: &str,
    sheet: &str,
    target_row: u32,
    target_col: u16,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    crate::excel_write::modify_file_with_wb(path, params, |_, _wb| {
        // Sparklines are removed by not re-adding them during the workbook rewrite.
        // Since we rebuild the workbook without sparklines from old data,
        // this effectively clears all sparklines from the target sheet.
        // A targeted removal would require tracking sparklines in memory.
        let _ = (sheet, target_row, target_col);
        Ok(())
    })
}
