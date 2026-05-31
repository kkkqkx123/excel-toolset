use std::collections::HashMap;

use crate::excel_read::read_all_sheets_to_map;
use crate::security::{compute_file_hash, create_backup};
use crate::types::*;

use super::csv::read_csv_to_cell_values;
use super::write::build_workbook_with_ops;

fn apply_data_operations(
    data: &mut HashMap<String, SheetData>,
    operations: &[BatchOperation],
) -> Result<usize> {
    let mut succeeded = 0usize;
    for op in operations {
        match op {
            BatchOperation::WriteCell {
                sheet,
                row,
                col,
                value,
            } => {
                super::cell::write(data, sheet, *row, *col, value)?;
                succeeded += 1;
            }
            BatchOperation::WriteRange {
                sheet,
                range,
                data: grid,
            } => {
                super::cell::write_range(data, sheet, range, grid)?;
                succeeded += 1;
            }
            BatchOperation::WriteRangeFromCsv {
                sheet,
                range,
                csv_path,
            } => {
                let grid = read_csv_to_cell_values(csv_path)?;
                super::cell::write_range(data, sheet, range, &grid)?;
                succeeded += 1;
            }
            BatchOperation::ClearRange { sheet, range } => {
                super::cell::clear_range(data, sheet, range)?;
                succeeded += 1;
            }
            BatchOperation::SetFormula {
                sheet,
                cell,
                formula,
            } => {
                super::cell::set_formula(data, sheet, cell, formula)?;
                succeeded += 1;
            }
            BatchOperation::InsertRows {
                sheet,
                at_row,
                data: grid,
            } => {
                super::cell::insert_rows(data, sheet, *at_row, grid)?;
                succeeded += 1;
            }
            BatchOperation::DeleteRows {
                sheet,
                start_row,
                end_row,
            } => {
                super::cell::delete_rows(data, sheet, *start_row, *end_row)?;
                succeeded += 1;
            }
            BatchOperation::AppendRows { sheet, data: grid } => {
                super::cell::append_rows(data, sheet, grid)?;
                succeeded += 1;
            }
            BatchOperation::AddSheet { name } => {
                super::sheet::add(data, name)?;
                succeeded += 1;
            }
            BatchOperation::DeleteSheet { name } => {
                super::sheet::delete(data, name)?;
                succeeded += 1;
            }
            BatchOperation::RenameSheet { old_name, new_name } => {
                super::sheet::rename(data, old_name, new_name)?;
                succeeded += 1;
            }
            BatchOperation::SortSheet { sheet, columns } => {
                super::sheet::sort(data, sheet, columns)?;
                succeeded += 1;
            }
            BatchOperation::DedupSheet { sheet, columns } => {
                super::sheet::dedup(data, sheet, columns)?;
                succeeded += 1;
            }
            BatchOperation::SetFormat { .. }
            | BatchOperation::MergeCells { .. }
            | BatchOperation::AddChart { .. } => {}
        }
    }
    Ok(succeeded)
}

pub fn execute_batch_operations(
    path: &str,
    params: &SecurityParams,
    operations: &[BatchOperation],
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
        });
    }

    let old_hash = compute_file_hash(path).map_err(AppError::Io)?;

    let backup_info = if params.create_backup {
        Some(create_backup(path, &old_hash).map_err(AppError::Io)?)
    } else {
        None
    };

    let mut data = read_all_sheets_to_map(path)?;

    let data_succeeded = apply_data_operations(&mut data, operations)?;

    let mut wb = build_workbook_with_ops(&data, operations)?;

    let all_ops_count = operations.len();
    let workbook_ops_count = operations
        .iter()
        .filter(|op| {
            matches!(
                op,
                BatchOperation::SetFormat { .. }
                    | BatchOperation::MergeCells { .. }
                    | BatchOperation::AddChart { .. }
            )
        })
        .count();
    let succeeded = data_succeeded + workbook_ops_count;

    let new_hash = if params.dry_run {
        old_hash.clone()
    } else {
        wb.save(path).map_err(AppError::Xlsx)?;
        compute_file_hash(path).map_err(AppError::Io)?
    };

    Ok(BatchWriteResult {
        success: true,
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
    })
}
