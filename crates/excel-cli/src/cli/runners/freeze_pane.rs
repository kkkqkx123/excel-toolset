use excel_core::excel_write;
use excel_core::types::*;
use crate::cli::args::*;

pub(crate) fn run_freeze_pane(args: &FreezePaneArgs) -> Result<serde_json::Value> {
    match &args.command {
        FreezePaneSub::Set {
            path,
            sheet,
            rows,
            cols,
            dry_run,
        } => {
            let config = FreezePanesConfig {
                sheet: sheet.clone(),
                rows: *rows,
                cols: *cols,
            };
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::set_freeze_panes(path, &params, &config)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        FreezePaneSub::Clear {
            path,
            sheet,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::clear_freeze_panes(path, &params, sheet)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

// ── AutoFilter ──

