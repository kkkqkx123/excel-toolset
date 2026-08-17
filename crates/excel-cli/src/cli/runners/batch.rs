use crate::cli::args::*;
use excel_core::excel_write;
use excel_core::types::*;
use excel_diff::semantic::{self, Verbosity};

pub(crate) fn run_batch(args: &BatchArgs, format: &str) -> Result<serde_json::Value> {
    match &args.command {
        BatchSub::Modify {
            path,
            operations,
            dry_run,
            strategy,
            validate_only,
        } => {
            let ops: Vec<BatchOperation> = serde_json::from_str(operations)
                .map_err(|e| AppError::Serialize(format!("Invalid operations JSON: {}", e)))?;
            let exec_strategy = if *validate_only {
                BatchExecutionStrategy::DryRun
            } else {
                match strategy.as_str() {
                    "all-or-nothing" => BatchExecutionStrategy::AllOrNothing,
                    "dry-run" => BatchExecutionStrategy::DryRun,
                    _ => BatchExecutionStrategy::BestEffort,
                }
            };
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let mut result = excel_write::execute_batch_operations_with_strategy(
                path,
                &params,
                &ops,
                &exec_strategy,
            )?;
            if let Some(ref backup) = result.backup_info
                && let Ok(diff) = excel_diff::diff_files(&backup.backup_path, path)
            {
                result.diff = Some(diff);
            }
            if format == "text" {
                let mut parts = Vec::new();
                if !result.message.is_empty() {
                    parts.push(result.message.clone());
                }
                if let Some(ref diff) = result.diff {
                    parts.push(semantic::to_natural_text(diff, None, Verbosity::Detail));
                }
                let text = if parts.is_empty() {
                    "Batch modify completed.".to_string()
                } else {
                    parts.join("\n")
                };
                Ok(serde_json::json!({"raw_text": text}))
            } else {
                Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
            }
        }
        BatchSub::ValidateRefs {
            path,
            sheet,
            formula,
        } => {
            let result = excel_write::validate_formula_references(path, sheet, formula)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}
