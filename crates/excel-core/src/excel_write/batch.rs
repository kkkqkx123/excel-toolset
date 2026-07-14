use std::collections::HashMap;

use regex::Regex;
use serde::Serialize;

use crate::excel_read::read_all_sheets_to_map;
use crate::security::{compute_file_hash, create_backup, rollback};
use crate::types::*;
use crate::utils::cell_ref;

use super::core::build_workbook_with_ops;
use super::csv::read_csv_to_cell_values;

pub fn validate_batch_operations(
    path: &str,
    operations: &[BatchOperation],
) -> Result<BatchValidateResult> {
    let sheets = crate::excel_read::list_sheets(path)?;
    let data = read_all_sheets_to_map(path)?;

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for (idx, op) in operations.iter().enumerate() {
        match op {
            BatchOperation::WriteCell {
                sheet, row, col, ..
            } => {
                if !sheets.contains(sheet) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::SheetNotFound,
                        message: format!("Sheet '{}' not found", sheet),
                    });
                } else if let Some(sheet_data) = data.get(sheet) {
                    if *row as usize >= sheet_data.rows.len()
                        || *col as usize >= sheet_data.rows.get(*row as usize).map(|r| r.len()).unwrap_or(0)
                    {
                        warnings.push(format!(
                            "WriteCell at {} row={} col={} is outside existing data range, will create new cell",
                            sheet, row, col
                        ));
                    }
                }
            }
            BatchOperation::WriteRange {
                sheet, range, data: grid,
            } => {
                if !sheets.contains(sheet) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::SheetNotFound,
                        message: format!("Sheet '{}' not found", sheet),
                    });
                }
                if let Err(e) = cell_ref::parse_range_normalized(range) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::RangeOutOfBounds,
                        message: format!("Invalid range '{}': {}", range, e),
                    });
                }
                if grid.is_empty() || grid.iter().all(|r| r.is_empty()) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::RangeOutOfBounds,
                        message: format!("WriteRange '{}': data cannot be empty", range),
                    });
                }
            }
            BatchOperation::WriteRangeFromCsv {
                sheet,
                range,
                csv_path,
            } => {
                if !sheets.contains(sheet) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::SheetNotFound,
                        message: format!("Sheet '{}' not found", sheet),
                    });
                }
                if let Err(e) = cell_ref::parse_range_normalized(range) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::RangeOutOfBounds,
                        message: format!("Invalid range '{}': {}", range, e),
                    });
                }
                if !std::path::Path::new(csv_path).exists() {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::CellNotFound,
                        message: format!("CSV file not found: {}", csv_path),
                    });
                }
            }
            BatchOperation::ClearRange { sheet, range } => {
                if !sheets.contains(sheet) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::SheetNotFound,
                        message: format!("Sheet '{}' not found", sheet),
                    });
                }
                if let Err(e) = cell_ref::parse_range_normalized(range) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::RangeOutOfBounds,
                        message: format!("Invalid range '{}': {}", range, e),
                    });
                }
            }
            BatchOperation::SetFormula {
                sheet,
                cell,
                formula,
            } => {
                if !sheets.contains(sheet) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::SheetNotFound,
                        message: format!("Sheet '{}' not found", sheet),
                    });
                }
                if let Err(e) = cell_ref::parse_cell_ref(cell) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::InvalidFormula,
                        message: format!("Invalid cell reference '{}': {}", cell, e),
                    });
                }
                if formula.trim().is_empty() {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::InvalidFormula,
                        message: "Formula cannot be empty".to_string(),
                    });
                }
            }
            BatchOperation::InsertRows {
                sheet, at_row, data: grid,
            } => {
                if !sheets.contains(sheet) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::SheetNotFound,
                        message: format!("Sheet '{}' not found", sheet),
                    });
                }
                if *at_row == 0 {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::RangeOutOfBounds,
                        message: "at_row must be >= 1 (1-indexed)".to_string(),
                    });
                }
                if grid.is_empty() {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::RangeOutOfBounds,
                        message: "InsertRows data cannot be empty".to_string(),
                    });
                }
            }
            BatchOperation::DeleteRows {
                sheet,
                start_row,
                end_row,
            } => {
                if !sheets.contains(sheet) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::SheetNotFound,
                        message: format!("Sheet '{}' not found", sheet),
                    });
                }
                if start_row > end_row {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::RangeOutOfBounds,
                        message: format!(
                            "start_row ({}) must be <= end_row ({})",
                            start_row, end_row
                        ),
                    });
                }
            }
            BatchOperation::AppendRows { sheet, data: grid } => {
                if !sheets.contains(sheet) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::SheetNotFound,
                        message: format!("Sheet '{}' not found", sheet),
                    });
                }
                if grid.is_empty() {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::RangeOutOfBounds,
                        message: "AppendRows data cannot be empty".to_string(),
                    });
                }
            }
            BatchOperation::AddSheet { name } => {
                if name.trim().is_empty() {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::InvalidFormula,
                        message: "Sheet name cannot be empty".to_string(),
                    });
                }
                if sheets.contains(name) {
                    warnings.push(format!("AddSheet '{}': sheet already exists", name));
                }
            }
            BatchOperation::DeleteSheet { name } => {
                if !sheets.contains(name) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::SheetNotFound,
                        message: format!("Sheet '{}' not found, cannot delete", name),
                    });
                }
            }
            BatchOperation::RenameSheet { old_name, new_name } => {
                if !sheets.contains(old_name) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::SheetNotFound,
                        message: format!("Sheet '{}' not found, cannot rename", old_name),
                    });
                }
                if new_name.trim().is_empty() {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::InvalidFormula,
                        message: "New sheet name cannot be empty".to_string(),
                    });
                }
            }
            BatchOperation::SortSheet { sheet, columns } => {
                if !sheets.contains(sheet) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::SheetNotFound,
                        message: format!("Sheet '{}' not found", sheet),
                    });
                }
                if columns.is_empty() {
                    warnings.push(format!(
                        "SortSheet '{}': no sort columns specified",
                        sheet
                    ));
                }
            }
            BatchOperation::DedupSheet { sheet, columns } => {
                if !sheets.contains(sheet) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::SheetNotFound,
                        message: format!("Sheet '{}' not found", sheet),
                    });
                }
                if columns.is_empty() {
                    warnings.push(format!(
                        "DedupSheet '{}': no dedup columns specified",
                        sheet
                    ));
                }
            }
            // Format/visual operations - only sheet validation
            BatchOperation::SetFormat { sheet, range, .. } => {
                if !sheets.contains(sheet) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::SheetNotFound,
                        message: format!("Sheet '{}' not found", sheet),
                    });
                }
                if let Err(e) = cell_ref::parse_range_normalized(range) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::RangeOutOfBounds,
                        message: format!("Invalid range '{}': {}", range, e),
                    });
                }
            }
            BatchOperation::MergeCells { sheet, range, .. } => {
                if !sheets.contains(sheet) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::SheetNotFound,
                        message: format!("Sheet '{}' not found", sheet),
                    });
                }
                if let Err(e) = cell_ref::parse_range_normalized(range) {
                    errors.push(ValidationError {
                        operation_index: idx,
                        error_type: ValidationErrorType::RangeOutOfBounds,
                        message: format!("Invalid range '{}': {}", range, e),
                    });
                }
            }
            // Chart/Table/Validation/Pivot/Sparkline operations don't need pre-validation
            _ => {}
        }
    }

    Ok(BatchValidateResult {
        valid: errors.is_empty(),
        errors,
        warnings,
    })
}

fn apply_data_operations(
    data: &mut HashMap<String, SheetData>,
    operations: &[BatchOperation],
) -> Result<(usize, Vec<FailedOperation>)> {
    let mut succeeded = 0usize;
    let mut failed = Vec::new();
    for (idx, op) in operations.iter().enumerate() {
        match op {
            BatchOperation::WriteCell {
                sheet,
                row,
                col,
                value,
            } => match super::data_mut::write(data, sheet, *row, *col, value) {
                Ok(()) => succeeded += 1,
                Err(e) => failed.push(FailedOperation {
                    operation_index: idx,
                    error: e.to_string(),
                    operation_type: "write_cell".to_string(),
                }),
            },
            BatchOperation::WriteRange {
                sheet,
                range,
                data: grid,
            } => match super::data_mut::write_range(data, sheet, range, grid) {
                Ok(()) => succeeded += 1,
                Err(e) => failed.push(FailedOperation {
                    operation_index: idx,
                    error: e.to_string(),
                    operation_type: "write_range".to_string(),
                }),
            },
            BatchOperation::WriteRangeFromCsv {
                sheet,
                range,
                csv_path,
            } => match read_csv_to_cell_values(csv_path) {
                Ok(grid) => match super::data_mut::write_range(data, sheet, range, &grid) {
                    Ok(()) => succeeded += 1,
                    Err(e) => failed.push(FailedOperation {
                        operation_index: idx,
                        error: e.to_string(),
                        operation_type: "write_range_from_csv".to_string(),
                    }),
                },
                Err(e) => failed.push(FailedOperation {
                    operation_index: idx,
                    error: e.to_string(),
                    operation_type: "write_range_from_csv".to_string(),
                }),
            },
            BatchOperation::ClearRange { sheet, range } => {
                match super::data_mut::clear_range(data, sheet, range) {
                    Ok(()) => succeeded += 1,
                    Err(e) => failed.push(FailedOperation {
                        operation_index: idx,
                        error: e.to_string(),
                        operation_type: "clear_range".to_string(),
                    }),
                }
            }
            BatchOperation::SetFormula {
                sheet,
                cell,
                formula,
            } => match super::data_mut::set_formula(data, sheet, cell, formula) {
                Ok(()) => succeeded += 1,
                Err(e) => failed.push(FailedOperation {
                    operation_index: idx,
                    error: e.to_string(),
                    operation_type: "set_formula".to_string(),
                }),
            },
            BatchOperation::InsertRows {
                sheet,
                at_row,
                data: grid,
            } => match super::data_mut::insert_rows(data, sheet, *at_row, grid) {
                Ok(()) => succeeded += 1,
                Err(e) => failed.push(FailedOperation {
                    operation_index: idx,
                    error: e.to_string(),
                    operation_type: "insert_rows".to_string(),
                }),
            },
            BatchOperation::DeleteRows {
                sheet,
                start_row,
                end_row,
            } => match super::data_mut::delete_rows(data, sheet, *start_row, *end_row) {
                Ok(()) => succeeded += 1,
                Err(e) => failed.push(FailedOperation {
                    operation_index: idx,
                    error: e.to_string(),
                    operation_type: "delete_rows".to_string(),
                }),
            },
            BatchOperation::AppendRows { sheet, data: grid } => {
                match super::data_mut::append_rows(data, sheet, grid) {
                    Ok(()) => succeeded += 1,
                    Err(e) => failed.push(FailedOperation {
                        operation_index: idx,
                        error: e.to_string(),
                        operation_type: "append_rows".to_string(),
                    }),
                }
            }
            BatchOperation::AddSheet { name } => match super::core::add(data, name) {
                Ok(()) => succeeded += 1,
                Err(e) => failed.push(FailedOperation {
                    operation_index: idx,
                    error: e.to_string(),
                    operation_type: "add_sheet".to_string(),
                }),
            },
            BatchOperation::DeleteSheet { name } => match super::core::delete(data, name) {
                Ok(()) => succeeded += 1,
                Err(e) => failed.push(FailedOperation {
                    operation_index: idx,
                    error: e.to_string(),
                    operation_type: "delete_sheet".to_string(),
                }),
            },
            BatchOperation::RenameSheet { old_name, new_name } => {
                match super::core::rename(data, old_name, new_name) {
                    Ok(()) => succeeded += 1,
                    Err(e) => failed.push(FailedOperation {
                        operation_index: idx,
                        error: e.to_string(),
                        operation_type: "rename_sheet".to_string(),
                    }),
                }
            }
            BatchOperation::SortSheet { sheet, columns } => {
                match super::core::sort(data, sheet, columns) {
                    Ok(()) => succeeded += 1,
                    Err(e) => failed.push(FailedOperation {
                        operation_index: idx,
                        error: e.to_string(),
                        operation_type: "sort_sheet".to_string(),
                    }),
                }
            }
            BatchOperation::DedupSheet { sheet, columns } => {
                match super::core::dedup(data, sheet, columns) {
                    Ok(()) => succeeded += 1,
                    Err(e) => failed.push(FailedOperation {
                        operation_index: idx,
                        error: e.to_string(),
                        operation_type: "dedup_sheet".to_string(),
                    }),
                }
            }
            BatchOperation::SetFormat { .. }
            | BatchOperation::MergeCells { .. }
            | BatchOperation::AddChart { .. }
            | BatchOperation::AddTable { .. }
            | BatchOperation::AddDataValidation { .. }
            | BatchOperation::AddPivotTable { .. }
            | BatchOperation::AddSparkline { .. }
            | BatchOperation::RemoveSparkline { .. } => {
                succeeded += 1;
            }
        }
    }
    Ok((succeeded, failed))
}

/// Execute batch operations with the given strategy.
/// Legacy callers can use execute_batch_operations which defaults to BestEffort.
pub fn execute_batch_operations_with_strategy(
    path: &str,
    params: &SecurityParams,
    operations: &[BatchOperation],
    strategy: &BatchExecutionStrategy,
) -> Result<BatchWriteResult> {
    if operations.is_empty() {
        let hash = compute_file_hash(path).map_err(AppError::Io)?;
        return Ok(BatchWriteResult {
            success: true,
            message: "No operations to execute".into(),
            backup_info: None,
            old_hash: hash.clone(),
            new_hash: hash,
            diff: None,
            operations_count: 0,
            succeeded_count: 0,
            failed_operations: None,
        });
    }

    let old_hash = compute_file_hash(path).map_err(AppError::Io)?;

    // DryRun: validate only, no execution
    if matches!(strategy, BatchExecutionStrategy::DryRun) {
        let validation = validate_batch_operations(path, operations)?;
        return Ok(BatchWriteResult {
            success: validation.valid,
            message: format!(
                "DryRun: validation {}. {} errors, {} warnings",
                if validation.valid { "passed" } else { "failed" },
                validation.errors.len(),
                validation.warnings.len()
            ),
            backup_info: None,
            old_hash: old_hash.clone(),
            new_hash: old_hash,
            diff: None,
            operations_count: operations.len(),
            succeeded_count: 0,
            failed_operations: None,
        });
    }

    // Create backup for AllOrNothing strategy
    let backup_info = if params.create_backup || matches!(strategy, BatchExecutionStrategy::AllOrNothing)
    {
        Some(create_backup(path, &old_hash).map_err(AppError::Io)?)
    } else {
        None
    };

    let all_ops_count = operations.len();

    let mut data = read_all_sheets_to_map(path)?;

    let (data_succeeded, failed_ops) = apply_data_operations(&mut data, operations)?;

    // Build workbook with all operations (including format/visual ops)
    let mut wb = match build_workbook_with_ops(&data, operations) {
        Ok(wb) => wb,
        Err(e) => {
            // If AllOrNothing with build failure, rollback
            if matches!(strategy, BatchExecutionStrategy::AllOrNothing) {
                if let Some(ref info) = backup_info {
                    let _ = rollback(info, path);
                }
            }
            return Err(e);
        }
    };

    let workbook_ops_count = operations
        .iter()
        .filter(|op| {
            matches!(
                op,
                BatchOperation::SetFormat { .. }
                    | BatchOperation::MergeCells { .. }
                    | BatchOperation::AddChart { .. }
                    | BatchOperation::AddTable { .. }
                    | BatchOperation::AddDataValidation { .. }
                    | BatchOperation::AddPivotTable { .. }
                    | BatchOperation::AddSparkline { .. }
                    | BatchOperation::RemoveSparkline { .. }
            )
        })
        .count();
    let succeeded = data_succeeded + workbook_ops_count;

    // AllOrNothing: rollback on any failure
    if matches!(strategy, BatchExecutionStrategy::AllOrNothing) && !failed_ops.is_empty() {
        if let Some(ref info) = backup_info {
            let _ = rollback(info, path);
        }
        let old_hash_clone = old_hash.clone();
        return Ok(BatchWriteResult {
            success: false,
            message: format!(
                "AllOrNothing: {}/{} operations failed, rolled back",
                failed_ops.len(),
                all_ops_count
            ),
            backup_info,
            old_hash: old_hash_clone,
            new_hash: old_hash, // rolled back, hash unchanged
            diff: None,
            operations_count: all_ops_count,
            succeeded_count: 0,
            failed_operations: Some(failed_ops),
        });
    }

    // Save the file
    let new_hash = if params.dry_run {
        old_hash.clone()
    } else {
        wb.save(path).map_err(AppError::Xlsx)?;
        compute_file_hash(path).map_err(AppError::Io)?
    };

    let success = failed_ops.is_empty();
    Ok(BatchWriteResult {
        success,
        message: format!(
            "Batch executed: {}/{} operations succeeded",
            succeeded, all_ops_count
        ),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
        operations_count: all_ops_count,
        succeeded_count: succeeded,
        failed_operations: if failed_ops.is_empty() {
            None
        } else {
            Some(failed_ops)
        },
    })
}

/// Execute batch operations with BestEffort strategy (backward-compatible default).
pub fn execute_batch_operations(
    path: &str,
    params: &SecurityParams,
    operations: &[BatchOperation],
) -> Result<BatchWriteResult> {
    execute_batch_operations_with_strategy(path, params, operations, &BatchExecutionStrategy::BestEffort)
}

/// Result of formula reference validation.
#[derive(Debug, Clone, Serialize)]
pub struct FormulaValidateResult {
    pub valid: bool,
    pub references: Vec<String>,
    pub invalid_references: Vec<String>,
}

/// Validate that all cell references in a formula point to existing sheets
/// and have syntactically valid cell coordinates.
///
/// Cross-sheet references (e.g. `Sheet2!A1`) are checked against the actual
/// sheet list of the workbook. Local references are checked for coordinate
/// validity only.
///
/// Range references like `A1:B5` are split into individual cell refs and
/// validated separately.
pub fn validate_formula_references(
    path: &str,
    sheet: &str,
    formula: &str,
) -> Result<FormulaValidateResult> {
    if formula.trim().is_empty() {
        return Ok(FormulaValidateResult {
            valid: true,
            references: vec![],
            invalid_references: vec![],
        });
    }

    let sheets = crate::excel_read::list_sheets(path)?;

    // Regex for cross-sheet refs: SheetName!CellRef or SheetName!CellRef:CellRef
    // Uses {1,3} column width to avoid matching function names
    let cross_re = Regex::new(
        r"([A-Za-z_][A-Za-z0-9_]*)!\$?[A-Za-z]{1,3}\$?\d+(?::\$?[A-Za-z]{1,3}\$?\d+)?",
    )
    .expect("invalid cross-sheet regex");

    // Regex for local refs: CellRef or CellRef:CellRef (range)
    let simple_re = Regex::new(
        r"(\$?[A-Za-z]{1,3}\$?\d+)(:\$?[A-Za-z]{1,3}\$?\d+)?",
    )
    .expect("invalid simple ref regex");

    let mut all_refs: Vec<String> = Vec::new();
    let mut invalid: Vec<String> = Vec::new();

    // --- Step 1: protect cross-sheet refs with placeholders ---
    // Collect all cross-sheet matches preserving order.
    // A cross-sheet ref may be a range (Sheet2!A1:B5) — both cells belong to
    // the cross-sheet and are recorded together.
    let mut cross_matches: Vec<(usize, usize, String, Vec<String>)> = Vec::new();
    // (match_start, match_end, sheet_name, cell_refs)
    for caps in cross_re.captures_iter(formula) {
        let m = caps.get(0).expect("no full match");
        let sheet_name = caps.get(1).expect("no sheet name").as_str().to_string();
        let full = m.as_str();
        let bang_pos = full.find('!').expect("no bang in cross ref");
        let rest = &full[bang_pos + 1..];

        let cell_refs = if let Some(colon_pos) = rest.find(':') {
            // Range: Sheet2!A1:B5 → ["A1", "B5"]
            let start = rest[..colon_pos].to_string();
            let end = rest[colon_pos + 1..].to_string();
            vec![start, end]
        } else {
            vec![rest.to_string()]
        };

        cross_matches.push((m.start(), m.end(), sheet_name, cell_refs));
    }

    // Build protected string
    let mut protected = String::with_capacity(formula.len());
    let mut last = 0usize;
    for (idx, (start, end, sheet_name, cell_refs)) in cross_matches.iter().enumerate() {
        protected.push_str(&formula[last..*start]);
        let placeholder = format!("%%{}%%", idx);
        protected.push_str(&placeholder);
        for cr in cell_refs {
            all_refs.push(format!("{}!{}", sheet_name, cr));
        }
        last = *end;
    }
    protected.push_str(&formula[last..]);

    // --- Step 2: extract local refs from protected string ---
    for caps in simple_re.captures_iter(&protected) {
        let full = caps.get(0).expect("no full match").as_str();
        if let Some(colon_pos) = full.find(':') {
            // Range reference: split at ':'
            let start_ref = &full[..colon_pos];
            let end_ref = &full[colon_pos + 1..];
            if !start_ref.is_empty() {
                all_refs.push(format!("{}!{}", sheet, start_ref));
            }
            if !end_ref.is_empty() {
                all_refs.push(format!("{}!{}", sheet, end_ref));
            }
        } else {
            all_refs.push(format!("{}!{}", sheet, full));
        }
    }

    // --- Step 3: validate all collected references ---
    // Excel limits: max row = 1,048,576 (0-indexed: 1,048,575),
    // max column = XFD = 16,384 (0-indexed: 16,383)
    const MAX_ROW: u32 = 1_048_575;
    const MAX_COL: u16 = 16_383;

    for ref_str in &all_refs {
        if let Some(bang_pos) = ref_str.find('!') {
            let s_name = &ref_str[..bang_pos];
            let cell = &ref_str[bang_pos + 1..];

            // Strip '$' for coordinate parsing
            let clean_cell = str::replace(cell, "$", "");

            match cell_ref::parse_cell_ref(&clean_cell) {
                Err(_e) => {
                    invalid.push(ref_str.clone());
                    continue;
                }
                Ok((row, col)) => {
                    if row > MAX_ROW || col > MAX_COL {
                        invalid.push(ref_str.clone());
                        continue;
                    }
                }
            }

            // Validate cross-sheet sheet name exists
            if s_name != sheet && !sheets.contains(&s_name.to_string()) {
                invalid.push(ref_str.clone());
            }
        }
    }

    Ok(FormulaValidateResult {
        valid: invalid.is_empty(),
        references: all_refs,
        invalid_references: invalid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: create a minimal xlsx file for validation tests
    fn with_test_file<F>(suffix: &str, f: F) -> Result<()>
    where
        F: FnOnce(&str) -> Result<()>,
    {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir
            .path()
            .join(format!("test_{}.xlsx", suffix))
            .to_str()
            .expect("path")
            .to_string();

        // Create a fresh minimal workbook with two sheets
        {
            let mut wb = rust_xlsxwriter::Workbook::new();
            wb.add_worksheet();
            let _ = wb.add_worksheet().set_name("Sheet2");
            wb.save(&path).expect("save");
        }

        f(&path)
    }

    #[test]
    fn test_validate_empty_formula() {
        with_test_file("empty", |path| {
            let result = validate_formula_references(path, "Sheet1", "").expect("validate");
            assert!(result.valid);
            assert!(result.references.is_empty());
            assert!(result.invalid_references.is_empty());
            Ok(())
        })
        .expect("test Ok");
    }

    #[test]
    fn test_validate_simple_formula() {
        with_test_file("simple", |path| {
            let result =
                validate_formula_references(path, "Sheet1", "A1 + B2").expect("validate");
            assert!(result.valid);
            assert_eq!(result.references.len(), 2);
            assert!(result.references.contains(&"Sheet1!A1".to_string()));
            assert!(result.references.contains(&"Sheet1!B2".to_string()));
            assert!(result.invalid_references.is_empty());
            Ok(())
        })
        .expect("test Ok");
    }

    #[test]
    fn test_validate_range_formula() {
        with_test_file("range", |path| {
            let result = validate_formula_references(path, "Sheet1", "SUM(A1:B5)")
                .expect("validate");
            assert!(result.valid);
            assert_eq!(result.references.len(), 2);
            assert!(result.references.contains(&"Sheet1!A1".to_string()));
            assert!(result.references.contains(&"Sheet1!B5".to_string()));
            assert!(result.invalid_references.is_empty());
            Ok(())
        })
        .expect("test Ok");
    }

    #[test]
    fn test_validate_cross_sheet_formula() {
        with_test_file("cross", |path| {
            let result =
                validate_formula_references(path, "Sheet1", "Sheet2!A1 + C3")
                    .expect("validate");
            assert!(result.valid);
            assert_eq!(result.references.len(), 2);
            assert!(result.references.contains(&"Sheet2!A1".to_string()));
            assert!(result.references.contains(&"Sheet1!C3".to_string()));
            assert!(result.invalid_references.is_empty());
            Ok(())
        })
        .expect("test Ok");
    }

    #[test]
    fn test_validate_cross_sheet_range_formula() {
        with_test_file("cross_range", |path| {
            let result =
                validate_formula_references(path, "Sheet1", "SUM(Sheet2!A1:B5)")
                    .expect("validate");
            assert!(result.valid);
            // Both A1 and B5 are captured as cross-sheet refs
            assert_eq!(result.references.len(), 2);
            assert!(result.references.contains(&"Sheet2!A1".to_string()));
            assert!(result.references.contains(&"Sheet2!B5".to_string()));
            assert!(result.invalid_references.is_empty());
            Ok(())
        })
        .expect("test Ok");
    }

    #[test]
    fn test_validate_nonexistent_sheet() {
        with_test_file("nosheet", |path| {
            let result = validate_formula_references(
                path,
                "Sheet1",
                "Sheet99!A1 + B2",
            )
            .expect("validate");
            assert!(!result.valid);
            assert_eq!(result.invalid_references.len(), 1);
            assert_eq!(result.invalid_references[0], "Sheet99!A1");
            // B2 is on current sheet (Sheet1), still valid
            Ok(())
        })
        .expect("test Ok");
    }

    #[test]
    fn test_validate_invalid_cell_ref() {
        with_test_file("badcell", |path| {
            // "ZZZ999999999" has an overflowing row number
            let result = validate_formula_references(
                path,
                "Sheet1",
                "ZZZ999999999",
            )
            .expect("validate");
            assert!(!result.valid);
            assert_eq!(result.invalid_references.len(), 1);
            Ok(())
        })
        .expect("test Ok");
    }

    #[test]
    fn test_validate_absolute_refs() {
        with_test_file("absolute", |path| {
            let result = validate_formula_references(
                path,
                "Sheet1",
                "$A$1 + $B2 + C$3",
            )
            .expect("validate");
            assert!(result.valid);
            assert_eq!(result.references.len(), 3);
            assert!(result.invalid_references.is_empty());
            Ok(())
        })
        .expect("test Ok");
    }

    #[test]
    fn test_validate_mixed_formula() {
        with_test_file("mixed", |path| {
            let result = validate_formula_references(
                path,
                "Sheet1",
                "SUM(Sheet2!$A$1:$B$5, C10, D20:E30)",
            )
            .expect("validate");
            assert!(result.valid);
            // Sheet2!$A$1:$B$5 → cross-sheet A1 + B5;
            // C10 → local; D20:E30 → local D20 + E30 = 5 refs
            assert_eq!(result.references.len(), 5);
            assert!(result.references.contains(&"Sheet2!$A$1".to_string()));
            assert!(result.references.contains(&"Sheet2!$B$5".to_string()));
            assert!(result.invalid_references.is_empty());
            Ok(())
        })
        .expect("test Ok");
    }
}
