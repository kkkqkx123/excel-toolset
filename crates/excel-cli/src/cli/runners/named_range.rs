use excel_core::features::named_ranges;
use excel_core::types::*;
use crate::cli::args::*;

pub(crate) fn run_named_range(args: &NamedRangeArgs) -> Result<serde_json::Value> {
    match &args.command {
        NamedRangeSub::List { path } => {
            let ranges = named_ranges::list_named_ranges(path)?;
            Ok(serde_json::to_value(ranges).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        NamedRangeSub::Get { path, name } => {
            let value = named_ranges::get_named_range_value(path, name)?;
            Ok(serde_json::to_value(value).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        NamedRangeSub::Create {
            path,
            name,
            range,
            sheet,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result =
                named_ranges::create_named_range(path, name, range, sheet.as_deref(), &params)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        NamedRangeSub::Delete {
            path,
            name,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = named_ranges::delete_named_range(path, name, &params)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

// ── Search ──

