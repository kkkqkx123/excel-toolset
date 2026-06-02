use crate::excel_data;
use crate::types::*;

#[cfg(feature = "sql")]
use crate::api::diff;
#[cfg(feature = "sql")]
use crate::excel_read;
#[cfg(feature = "sql")]
use crate::security;
#[cfg(feature = "sql")]
use rust_xlsxwriter::{Workbook, Worksheet};

#[cfg(feature = "sql")]
fn write_workbook(path: &str, data: &std::collections::HashMap<String, SheetData>) -> Result<()> {
    let mut wb = Workbook::new();
    for (name, sd) in data {
        let ws = wb.add_worksheet();
        ws.set_name(name).map_err(AppError::Xlsx)?;
        excel_data::write_sheet_data(ws, sd)?;
    }
    wb.save(path).map_err(AppError::Xlsx)?;
    Ok(())
}

#[cfg(not(feature = "sql"))]
pub fn filter_rows(
    path: &str,
    sheet: &str,
    conditions: &[FilterCondition],
) -> Result<Vec<Vec<CellData>>> {
    excel_data::filter_rows(path, sheet, conditions)
}

#[cfg(feature = "sql")]
pub fn filter_rows(
    path: &str,
    sheet: &str,
    conditions: &[FilterCondition],
) -> Result<Vec<Vec<CellData>>> {
    let data = excel_read::read_sheet_all(path, sheet)?;
    let result = excel_sql::filter_rows_on_data(&data, sheet, conditions, true)?;
    Ok(result.rows)
}

#[cfg(not(feature = "sql"))]
pub fn sort_sheet(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    sort_columns: &[SortColumn],
) -> Result<WriteResult> {
    excel_data::sort_sheet(path, params, sheet, sort_columns)
}

#[cfg(feature = "sql")]
pub fn sort_sheet(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    sort_columns: &[SortColumn],
) -> Result<WriteResult> {
    let old_data = excel_read::read_all_sheets_to_map(path)?;
    let old_sheet = old_data
        .get(sheet)
        .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?
        .clone();

    let old_hash = security::compute_file_hash(path).map_err(AppError::Io)?;
    let backup_info = if params.create_backup {
        Some(security::create_backup(path, &old_hash).map_err(AppError::Io)?)
    } else {
        None
    };

    let new_sheet = excel_sql::sort_sheet_on_data(&old_sheet, sort_columns)?;

    let new_hash = if params.dry_run {
        old_hash.clone()
    } else {
        let mut new_data = old_data.clone();
        new_data.insert(sheet.to_string(), new_sheet.clone());
        write_workbook(path, &new_data)?;
        security::compute_file_hash(path).map_err(AppError::Io)?
    };

    let cell_diffs = diff::compute_cell_diffs(&old_sheet, &new_sheet);
    let diff = if cell_diffs.is_empty() {
        None
    } else {
        let sheet_diff = SheetDiff {
            sheet_name: sheet.to_string(),
            row_count_diff: new_sheet.rows.len() as i32 - old_sheet.rows.len() as i32,
            col_count_diff: 0,
            cell_diffs,
        };
        let summary = diff::make_diff_summary(&sheet_diff.cell_diffs);
        Some(FileDiff {
            file_hash_match: old_hash == new_hash,
            sheet_diffs: vec![sheet_diff],
            summary,
        })
    };

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff,
    })
}

#[cfg(not(feature = "sql"))]
pub fn dedup_sheet(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    columns: &[u16],
) -> Result<WriteResult> {
    excel_data::dedup_sheet(path, params, sheet, columns)
}

#[cfg(feature = "sql")]
pub fn dedup_sheet(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    columns: &[u16],
) -> Result<WriteResult> {
    let old_data = excel_read::read_all_sheets_to_map(path)?;
    let old_sheet = old_data
        .get(sheet)
        .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?
        .clone();

    let old_hash = security::compute_file_hash(path).map_err(AppError::Io)?;
    let backup_info = if params.create_backup {
        Some(security::create_backup(path, &old_hash).map_err(AppError::Io)?)
    } else {
        None
    };

    let new_sheet = excel_sql::dedup_sheet_on_data(&old_sheet, columns)?;

    let new_hash = if params.dry_run {
        old_hash.clone()
    } else {
        let mut new_data = old_data.clone();
        new_data.insert(sheet.to_string(), new_sheet.clone());
        write_workbook(path, &new_data)?;
        security::compute_file_hash(path).map_err(AppError::Io)?
    };

    let cell_diffs = diff::compute_cell_diffs(&old_sheet, &new_sheet);
    let diff = if cell_diffs.is_empty() {
        None
    } else {
        let sheet_diff = SheetDiff {
            sheet_name: sheet.to_string(),
            row_count_diff: new_sheet.rows.len() as i32 - old_sheet.rows.len() as i32,
            col_count_diff: 0,
            cell_diffs,
        };
        let summary = diff::make_diff_summary(&sheet_diff.cell_diffs);
        Some(FileDiff {
            file_hash_match: old_hash == new_hash,
            sheet_diffs: vec![sheet_diff],
            summary,
        })
    };

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff,
    })
}

#[cfg(feature = "sql")]
pub fn sql_query(path: &str, _sheet: &str, query: &str) -> Result<Vec<Vec<CellData>>> {
    let data = excel_read::read_all_sheets_to_map(path)?;
    let sheets: Vec<SheetData> = data.into_values().collect();
    let result = excel_sql::sql_query_on_data(&sheets, query, true)?;
    Ok(result.rows)
}

#[cfg(not(feature = "sql"))]
pub fn sql_query(_path: &str, _sheet: &str, _query: &str) -> Result<Vec<Vec<CellData>>> {
    Err(AppError::FeatureNotEnabled(
        "SQL queries require the 'sql' feature (enable with --features sql)".into(),
    ))
}