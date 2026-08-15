use excel_core::excel_write;
use excel_core::types::*;
use crate::cli::args::*;

pub(crate) fn run_page_setup(args: &PageSetupArgs) -> Result<serde_json::Value> {
    match &args.command {
        PageSetupSub::Configure {
            path,
            sheet,
            config,
            dry_run,
        } => {
            let mut page_config: PageSetupConfig = serde_json::from_str(config).map_err(|e| {
                AppError::Serialize(format!("Invalid page setup config JSON: {}", e))
            })?;
            page_config.sheet = sheet.clone();
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::configure_page_setup(path, &params, &page_config)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        PageSetupSub::PageBreaks {
            path,
            config,
            dry_run,
        } => {
            let pb_config: PageBreakConfig = serde_json::from_str(config).map_err(|e| {
                AppError::Serialize(format!("Invalid page break config JSON: {}", e))
            })?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::set_page_breaks(path, &params, &pb_config)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        PageSetupSub::ClearBreaks {
            path,
            sheet,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::clear_page_breaks(path, &params, sheet)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

// ── Image ──

