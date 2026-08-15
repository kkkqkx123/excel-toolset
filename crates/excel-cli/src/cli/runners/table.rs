use excel_core::excel_write;
use excel_core::types::*;
use crate::cli::args::*;

pub(crate) fn run_table(args: &TableArgs) -> Result<serde_json::Value> {
    match &args.command {
        TableSub::Create {
            path,
            config,
            dry_run,
        } => {
            let table_config: TableConfig = serde_json::from_str(config)
                .map_err(|e| AppError::Serialize(format!("Invalid table config JSON: {}", e)))?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::create_table(path, &params, &table_config)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        TableSub::Remove {
            path,
            name,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::remove_table(path, &params, name)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        TableSub::List { path } => {
            let tables = excel_core::features::table::list_tables(path)?;
            Ok(serde_json::to_value(tables).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        TableSub::Get { path, name } => {
            let table = excel_core::features::table::get_table(path, name)?;
            Ok(serde_json::to_value(table).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

// ── Data Validation ──

