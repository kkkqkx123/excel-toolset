use excel_core::features::vba_util;
use excel_core::types::*;
use crate::cli::args::*;

pub(crate) fn run_vba(args: &VbaArgs) -> Result<serde_json::Value> {
    match &args.command {
        VbaSub::Export { path, output } => {
            let data = vba_util::export_vba(path)?;
            std::fs::write(output, &data)?;
            Ok(serde_json::json!({
                "success": true,
                "message": format!("VBA exported to {}", output)
            }))
        }
        VbaSub::Import {
            path,
            vba_file,
            dry_run,
        } => {
            let data = std::fs::read(vba_file)?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = vba_util::import_vba(path, &params, &data)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        VbaSub::Has { path } => {
            let has = vba_util::has_vba(path)?;
            Ok(serde_json::json!({
                "success": true,
                "has_vba": has
            }))
        }
    }
}

