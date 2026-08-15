use excel_core::excel_write;
use excel_core::types::*;
use crate::cli::args::*;

pub(crate) fn run_pivot_table(args: &PivotTableArgs) -> Result<serde_json::Value> {
    match &args.command {
        PivotTableSub::Create {
            path,
            config,
            dry_run,
        } => {
            let pt_config: PivotTableConfig = serde_json::from_str(config).map_err(|e| {
                AppError::Serialize(format!("Invalid pivot table config JSON: {}", e))
            })?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::create_pivot_table(path, &params, &pt_config)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

// ── Slicer ──

