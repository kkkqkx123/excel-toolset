use excel_core::excel_write;
use excel_core::types::*;
use crate::cli::args::*;

pub(crate) fn run_data_validation(args: &DataValidationArgs) -> Result<serde_json::Value> {
    match &args.command {
        DataValidationSub::Add {
            path,
            sheet,
            config,
            dry_run,
        } => {
            let dv_config: DataValidationConfig = serde_json::from_str(config).map_err(|e| {
                AppError::Serialize(format!("Invalid data validation config JSON: {}", e))
            })?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::add_data_validation(path, &params, sheet, &dv_config)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        DataValidationSub::Remove {
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
            let result = excel_write::remove_data_validation(path, &params, sheet, range)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

// ── Pivot Table ──

