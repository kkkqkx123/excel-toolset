use crate::cli::args::*;
use chrono::Utc;
use excel_core::security;
use excel_core::types::*;

pub(crate) fn run_rollback(args: &RollbackArgs) -> Result<serde_json::Value> {
    let hash = security::compute_file_hash(&args.backup_path).map_err(AppError::Io)?;
    let backup = BackupInfo {
        backup_path: args.backup_path.clone(),
        timestamp: Utc::now(),
        operation: "rollback".into(),
        file_hash: hash,
    };
    security::rollback(&backup, &args.path)?;
    Ok(serde_json::json!({
        "success": true,
        "message": format!("Rolled back {} from {}", args.path, args.backup_path)
    }))
}

// ── Comments ──
