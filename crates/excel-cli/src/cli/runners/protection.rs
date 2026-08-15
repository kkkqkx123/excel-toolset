use excel_core::excel_write;
use excel_core::types::*;
use crate::cli::args::*;

pub(crate) fn run_protection(args: &ProtectionArgs) -> Result<serde_json::Value> {
    match &args.command {
        ProtectionSub::Protect {
            path,
            sheet,
            password,
            options,
            dry_run,
        } => {
            let protection_options = if let Some(opts_json) = options {
                serde_json::from_str::<ProtectionOptions>(opts_json)
                    .map_err(|e| AppError::Serialize(format!("Invalid options JSON: {}", e)))?
            } else {
                ProtectionOptions::default()
            };
            let config = SheetProtectionConfig {
                sheet: sheet.clone(),
                password: password.clone(),
                options: protection_options,
            };
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::protect_sheet(path, &params, &config)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        ProtectionSub::Unprotect {
            path,
            sheet,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::unprotect_sheet(path, &params, sheet)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        ProtectionSub::IsProtected { path, sheet } => {
            let protected = excel_write::is_sheet_protected(path, sheet)?;
            Ok(serde_json::json!({
                "success": true,
                "sheet": sheet,
                "protected": protected
            }))
        }
    }
}

// ── Page Setup ──

