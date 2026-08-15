use excel_core::excel_write;
use excel_core::types::*;
use crate::cli::args::*;

pub(crate) fn run_slicer(args: &SlicerArgs) -> Result<serde_json::Value> {
    match &args.command {
        SlicerSub::Create {
            path,
            config,
            dry_run,
        } => {
            let slicer_config: SlicerConfig = serde_json::from_str(config)
                .map_err(|e| AppError::Serialize(format!("Invalid slicer config JSON: {}", e)))?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::create_slicer(path, &params, &slicer_config)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

// ── Overview / History ──

