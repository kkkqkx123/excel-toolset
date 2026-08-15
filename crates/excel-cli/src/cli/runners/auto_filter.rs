use excel_core::excel_write;
use excel_core::types::*;
use crate::cli::args::*;

pub(crate) fn run_auto_filter(args: &AutoFilterArgs) -> Result<serde_json::Value> {
    match &args.command {
        AutoFilterSub::Set {
            path,
            sheet,
            range,
            dry_run,
        } => {
            let config = AutoFilterConfig {
                sheet: sheet.clone(),
                range: range.clone(),
            };
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::set_auto_filter(path, &params, &config)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        AutoFilterSub::Remove {
            path,
            sheet,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::remove_auto_filter(path, &params, sheet)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        AutoFilterSub::Get { path, sheet } => {
            let info = excel_write::get_auto_filter(path, sheet)?;
            Ok(serde_json::to_value(info).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

// ── Protection ──

