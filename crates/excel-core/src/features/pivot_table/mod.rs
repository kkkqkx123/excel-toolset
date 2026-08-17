//! Pivot "table" feature implementation.
//!
//! **This does not produce a native Excel PivotTable.** rust_xlsxwriter cannot
//! emit `xl/pivotTables/*.xml`, so there is no pivot cache, no field list and
//! no interactive refresh. What we do instead is read the source range,
//! aggregate it in memory, and write the resulting **flattened summary table**
//! into the target sheet as ordinary cells.
//!
//! The distinction matters: callers previously got `success: true` and assumed
//! a real pivot part existed. Every successful result now says so explicitly in
//! `WriteResult::message`.

use crate::excel_read;
use crate::security;
use crate::types::*;

/// Aggregate data by row/column fields and create a pivot table.
pub fn create_pivot_table(
    path: &str,
    config: &PivotTableConfig,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    let (source_sheet, source_range) = parse_source_range(&config.source_range)?;

    let source_data = excel_read::read_range(path, &source_sheet, &source_range)?;

    if source_data.is_empty() {
        return Err(AppError::InvalidInput(
            "Source data is empty for pivot table".to_string(),
        ));
    }

    let headers: Vec<String> = source_data[0]
        .iter()
        .map(|c| c.value.clone().unwrap_or_default())
        .collect();
    let data_rows: Vec<&Vec<CellData>> = source_data[1..].iter().collect();

    // Apply date grouping if configured
    let (mut adjusted_headers, mut adjusted_rows) =
        apply_date_grouping(config, &headers, &data_rows);

    // Process calculated fields: evaluate formulas and append as new columns
    let calc_field_columns =
        process_calculated_fields(config, &mut adjusted_headers, &mut adjusted_rows)?;

    let pivot_data = build_pivot_data(
        config,
        &adjusted_headers,
        &adjusted_rows,
        &calc_field_columns,
    )?;

    let (target_r, target_c) = crate::utils::cell_ref::parse_cell_ref(&config.target_cell)?;

    let params_for_write = SecurityParams {
        file_path: path.to_string(),
        ..params.clone()
    };

    let rows_written = pivot_data.len();
    let cols_written = pivot_data.iter().map(|r| r.len()).max().unwrap_or(0);

    let mut result = crate::excel_write::modify_file_with_wb(path, &params_for_write, |_, wb| {
        let worksheet = wb
            .worksheet_from_name(&config.target_sheet)
            .map_err(|_e| AppError::SheetNotFound(config.target_sheet.clone()))?;

        let mut row = target_r;

        for data_row in &pivot_data {
            let mut col = target_c;
            for cell_value in data_row {
                write_cell_value_to_worksheet(worksheet, row, col, cell_value)?;
                col += 1;
            }
            row += 1;
        }

        Ok(())
    })?;

    // C3: do not claim we created a native PivotTable — we did not.
    result.message = format!(
        "Wrote a flattened aggregate summary ({} rows x {} cols) to '{}'!{}. \
         NOTE: this is NOT a native Excel PivotTable — no pivot cache or field \
         list is created, and the cells will not refresh when the source data \
         changes.",
        rows_written, cols_written, config.target_sheet, config.target_cell
    );

    Ok(result)
}

// ── 子模块：聚合 / 日期分组 / 计算字段表达式引擎 ──
mod aggregate;
mod calc_field;
mod grouping;

use self::{aggregate::*, calc_field::*, grouping::*};

#[cfg(test)]
mod tests;
