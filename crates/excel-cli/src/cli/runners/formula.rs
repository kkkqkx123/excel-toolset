use crate::cli::args::*;
use excel_core::excel_read;
use excel_core::excel_write;
use excel_core::features::formula_analysis;
use excel_core::features::formula_eval;
use excel_core::features::formula_ops;
use excel_core::types::*;

pub(crate) fn run_formula(args: &FormulaArgs) -> Result<serde_json::Value> {
    match &args.command {
        FormulaSub::Set {
            path,
            sheet,
            cell,
            formula,
            eval,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            if *eval {
                let result =
                    formula_eval::set_formula_with_eval(path, sheet, cell, formula, true, &params)?;
                Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
            } else {
                let result = excel_write::set_formula(path, &params, sheet, cell, formula)?;
                Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
            }
        }
        FormulaSub::Refresh {
            path,
            sheet,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::refresh_formulas(path, &params, sheet)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        FormulaSub::Read { path, sheet, cell } => {
            let formula = excel_read::read_formula(path, sheet, cell)?;
            Ok(serde_json::json!({
                "success": true,
                "formula": formula
            }))
        }
        FormulaSub::CalcMode {
            path,
            mode,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::set_calculation_mode(path, &params, mode)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        FormulaSub::Trace { path, sheet, cell } => {
            let trace = formula_analysis::trace_dependencies(path, sheet, cell)?;
            Ok(serde_json::to_value(trace).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        FormulaSub::Explain {
            path,
            sheet,
            cell,
            language,
        } => {
            let explanation = formula_analysis::explain_formula(path, sheet, cell, language)?;
            Ok(
                serde_json::to_value(explanation)
                    .map_err(|e| AppError::Serialize(e.to_string()))?,
            )
        }
        FormulaSub::ExplainLogic {
            path,
            sheet,
            cell,
            language,
        } => {
            let logic = formula_analysis::explain_formula_logic(path, sheet, cell, language)?;
            Ok(serde_json::to_value(logic).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        FormulaSub::Fill {
            path,
            sheet,
            source,
            target_range,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = formula_ops::fill_formula(path, sheet, source, target_range, &params)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        FormulaSub::Eval {
            path,
            sheet,
            cell,
            formula,
            no_eval,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let evaluate = !no_eval;
            let result =
                formula_eval::set_formula_with_eval(path, sheet, cell, formula, evaluate, &params)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        FormulaSub::EvalBatch {
            path,
            sheet,
            formulas,
            dry_run,
        } => {
            let parsed: serde_json::Value = serde_json::from_str(formulas)
                .map_err(|e| AppError::Serialize(format!("Invalid formulas JSON: {}", e)))?;

            let entries = parsed
                .as_array()
                .ok_or_else(|| AppError::Serialize("formulas must be a JSON array".to_string()))?;

            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };

            let mut results = Vec::new();
            for entry in entries {
                let cell = entry["cell"].as_str().ok_or_else(|| {
                    AppError::Serialize("each entry must have a 'cell' field".to_string())
                })?;
                let formula = entry["formula"].as_str().ok_or_else(|| {
                    AppError::Serialize("each entry must have a 'formula' field".to_string())
                })?;

                let result =
                    formula_eval::set_formula_with_eval(path, sheet, cell, formula, true, &params)?;
                results.push(serde_json::json!({
                    "cell": cell,
                    "formula": formula,
                    "message": result.message,
                    "success": true,
                }));
            }

            Ok(serde_json::json!({
                "success": true,
                "count": results.len(),
                "results": results,
            }))
        }
    }
}
