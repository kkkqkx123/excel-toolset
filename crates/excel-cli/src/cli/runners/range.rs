use crate::cli::args::*;
use excel_core::excel_read;
use excel_core::excel_write;
use excel_core::types::*;
use excel_core::utils::helpers;

pub(crate) fn run_range(args: &RangeArgs) -> Result<serde_json::Value> {
    match &args.command {
        RangeSub::Read {
            path,
            sheet,
            range,
            mode,
            truncate,
        } => {
            let output_mode = match mode.as_str() {
                "compact" => OutputMode::Compact,
                "csv" => OutputMode::Csv,
                _ => OutputMode::Detailed,
            };
            let options = ReadRangeOptions {
                mode: output_mode,
                truncate: *truncate,
                include_context: Some(false),
                context_size: Some(3),
            };
            let data = excel_read::read_range_with_options(path, sheet, range, &options)?;
            Ok(serde_json::to_value(data).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        RangeSub::Write {
            path,
            sheet,
            range,
            data,
            dry_run,
        } => {
            let values: Vec<Vec<CellValue>> = helpers::parse_cell_value_grid(data)?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::write_range(path, &params, sheet, range, &values)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        RangeSub::Clear {
            path,
            sheet,
            range,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::clear_range(path, &params, sheet, range)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        RangeSub::WriteCsv {
            path,
            sheet,
            range,
            csv,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::write_range_from_csv(path, &params, sheet, range, csv)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}
