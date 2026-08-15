use excel_core::excel_write;
use excel_core::features::sparkline;
use excel_core::types::*;
use crate::cli::args::*;

pub(crate) fn run_sparkline(args: &SparklineArgs) -> Result<serde_json::Value> {
    match &args.command {
        SparklineSub::Add {
            path,
            sheet,
            source_range,
            sparkline_type,
            target_cell,
            style,
            dry_run,
        } => {
            let (target_row, target_col) =
                excel_core::utils::cell_ref::parse_cell_ref(target_cell)?;
            let st = sparkline::parse_sparkline_type(sparkline_type);
            let config = SparklineConfig {
                sparkline_type: st,
                sheet: sheet.clone(),
                source_range: source_range.clone(),
                target_row,
                target_col,
                style: *style,
            };
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::add_sparkline(path, &params, &config)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        SparklineSub::Remove {
            path,
            sheet,
            target_cell,
            dry_run,
        } => {
            let (target_row, target_col) =
                excel_core::utils::cell_ref::parse_cell_ref(target_cell)?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result =
                excel_write::remove_sparkline(path, &params, sheet, target_row, target_col)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

