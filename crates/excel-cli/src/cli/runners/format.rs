use excel_core::excel_write;
use excel_core::types::*;
use crate::cli::args::*;

pub(crate) fn run_format(args: &FormatArgs) -> Result<serde_json::Value> {
    match &args.command {
        FormatSub::Set {
            path,
            sheet,
            range,
            style,
            dry_run,
        } => {
            let style_val: Style = serde_json::from_str(style)
                .map_err(|e| AppError::Serialize(format!("Invalid style JSON: {}", e)))?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::set_format(path, &params, sheet, range, &style_val)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        FormatSub::Merge {
            path,
            sheet,
            range,
            value,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let merge_value = value.as_deref().unwrap_or("");
            let result = excel_write::merge_cells(path, &params, sheet, range, merge_value)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

