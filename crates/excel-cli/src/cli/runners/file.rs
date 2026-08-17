use crate::cli::args::*;
use excel_core::excel_read;
use excel_core::excel_write;
use excel_core::security;
use excel_core::types::*;

pub(crate) fn run_file(args: &FileArgs) -> Result<serde_json::Value> {
    match &args.command {
        FileSub::Create { path, sheet } => {
            let result = excel_write::create_file(path, sheet)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        FileSub::Info { path } => {
            let info = excel_read::read_file_info(path)?;
            Ok(serde_json::to_value(info).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        FileSub::Backup { path, output } => {
            let hash = security::compute_file_hash(path)?;
            let backup = security::create_backup(path, &hash)?;
            if let Some(out) = output {
                std::fs::copy(&backup.backup_path, out)?;
            }
            Ok(serde_json::json!({
                "success": true,
                "backup_path": backup.backup_path,
                "timestamp": backup.timestamp,
                "file_hash": backup.file_hash
            }))
        }
    }
}
