use crate::cli::args::*;
use excel_core::excel_read;
use excel_core::excel_write;
use excel_core::types::*;

pub(crate) fn run_sheet(args: &SheetArgs) -> Result<serde_json::Value> {
    match &args.command {
        SheetSub::List { path } => {
            let sheets = excel_read::list_sheets(path)?;
            Ok(serde_json::json!({ "success": true, "sheets": sheets }))
        }
        SheetSub::Add { path, name } => {
            let params = SecurityParams {
                dry_run: false,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::add_sheet(path, &params, name)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        SheetSub::Delete { path, name } => {
            let params = SecurityParams {
                dry_run: false,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::delete_sheet(path, &params, name)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        SheetSub::Rename { path, old, new } => {
            let params = SecurityParams {
                dry_run: false,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::rename_sheet(path, &params, old, new)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        SheetSub::SetVisibility {
            path,
            name,
            visibility,
            dry_run,
        } => {
            let vis: SheetVisibility = serde_json::from_str(&format!("\"{visibility}\""))
                .map_err(|e| AppError::Serialize(format!("Invalid visibility value: {e}")))?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::set_sheet_visibility(path, name, &vis, &params)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}
