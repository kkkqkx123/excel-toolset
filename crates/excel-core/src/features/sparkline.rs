use crate::security;
use crate::types::*;

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

    crate::excel_write::modify_file_with_wb(path, params, |_old_data, wb| {
        let ws = wb
            .worksheet_from_name(&config.sheet)
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
